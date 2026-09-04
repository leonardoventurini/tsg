use std::collections::{HashSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use fs2::FileExt;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};

use crate::error::{Error, Result};
use crate::types::{
    CommitReceipt, Direction, Durability, Node, SearchBackend, SearchFilter, SearchResults,
    StoreStats, WriteBatch,
};
use crate::vector::{encode_vector, exact_search, VectorAccelerator};

const CURRENT_SCHEMA_VERSION: u32 = 1;
const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS store_metadata (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    dimensions INTEGER NOT NULL,
    generation INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS nodes (
    key INTEGER PRIMARY KEY,
    id TEXT NOT NULL UNIQUE,
    repository_id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    content TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS nodes_repository_kind
ON nodes(repository_id, kind, key);

CREATE TABLE IF NOT EXISTS edges (
    source_key INTEGER NOT NULL REFERENCES nodes(key) ON DELETE CASCADE,
    target_key INTEGER NOT NULL REFERENCES nodes(key) ON DELETE CASCADE,
    relationship TEXT NOT NULL,
    PRIMARY KEY (source_key, relationship, target_key)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS edges_target
ON edges(target_key, relationship, source_key);

CREATE TABLE IF NOT EXISTS embeddings (
    node_key INTEGER PRIMARY KEY REFERENCES nodes(key) ON DELETE CASCADE,
    vector BLOB NOT NULL
);
";

pub struct StoreBuilder {
    database_path: PathBuf,
    dimensions: usize,
    exact_search_threshold: usize,
    durability: Durability,
    read_only: bool,
}

impl StoreBuilder {
    #[must_use]
    pub fn new(database_path: impl Into<PathBuf>, dimensions: usize) -> Self {
        Self {
            database_path: database_path.into(),
            dimensions,
            exact_search_threshold: 10_000,
            durability: Durability::Full,
            read_only: false,
        }
    }

    #[must_use]
    pub fn exact_search_threshold(mut self, threshold: usize) -> Self {
        self.exact_search_threshold = threshold;
        self
    }

    #[must_use]
    pub fn durability(mut self, durability: Durability) -> Self {
        self.durability = durability;
        self
    }

    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Opens the configured store.
    ///
    /// # Errors
    ///
    /// Returns a typed error for invalid configuration, lock contention, schema
    /// incompatibility, or storage initialization failure.
    pub fn build(self) -> Result<Store> {
        Store::open_builder(&self)
    }
}

pub struct Store {
    connection: Connection,
    dimensions: usize,
    exact_search_threshold: usize,
    accelerator: Mutex<Option<VectorAccelerator>>,
    vector_path: PathBuf,
    durability: Durability,
    read_only: bool,
    _writer_lock: Option<File>,
}

impl Store {
    /// Opens or creates a TSG store and reconstructs its vector accelerator when needed.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid dimensions, incompatible existing stores, or
    /// filesystem and `SQLite` failures. Accelerator failures degrade to exact search.
    pub fn open(
        database_path: impl AsRef<Path>,
        dimensions: usize,
        exact_search_threshold: usize,
    ) -> Result<Self> {
        StoreBuilder::new(database_path.as_ref().to_path_buf(), dimensions)
            .exact_search_threshold(exact_search_threshold)
            .build()
    }

    fn open_builder(builder: &StoreBuilder) -> Result<Self> {
        if builder.dimensions == 0 {
            return Err(Error::InvalidInput(
                "embedding dimensions must be positive".to_string(),
            ));
        }
        let database_path = &builder.database_path;
        if !builder.read_only {
            if let Some(parent) = database_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        if builder.read_only && !database_path.exists() {
            return Err(Error::InvalidInput(format!(
                "read-only database does not exist: {}",
                database_path.display()
            )));
        }

        let writer_lock = if builder.read_only {
            None
        } else {
            Some(acquire_writer_lock(database_path)?)
        };
        let flags = if builder.read_only {
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
        };
        let connection = Connection::open_with_flags(database_path, flags)?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        if builder.read_only {
            validate_schema(&connection)?;
        } else {
            connection.execute_batch("PRAGMA journal_mode = WAL;")?;
            match builder.durability {
                Durability::Full => connection.execute_batch("PRAGMA synchronous = FULL;")?,
                Durability::Normal => connection.execute_batch("PRAGMA synchronous = NORMAL;")?,
            }
            initialize_schema(&connection, builder.dimensions)?;
        }

        let (stored_dimensions, generation): (usize, u64) = connection.query_row(
            "SELECT dimensions, generation FROM store_metadata WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if stored_dimensions != builder.dimensions {
            return Err(Error::InvalidInput(format!(
                "embedding dimension mismatch: stored {stored_dimensions}, requested {}",
                builder.dimensions
            )));
        }

        let vector_path = database_path.with_extension("usearch");
        let accelerator = if builder.read_only {
            VectorAccelerator::open_existing(vector_path.clone(), builder.dimensions, generation)
                .ok()
        } else {
            VectorAccelerator::open_or_rebuild(
                &connection,
                vector_path.clone(),
                builder.dimensions,
                generation,
                builder.durability,
            )
            .ok()
        };

        Ok(Self {
            connection,
            dimensions: builder.dimensions,
            exact_search_threshold: builder.exact_search_threshold,
            accelerator: Mutex::new(accelerator),
            vector_path,
            durability: builder.durability,
            read_only: builder.read_only,
            _writer_lock: writer_lock,
        })
    }

    #[must_use]
    pub fn builder(database_path: impl Into<PathBuf>, dimensions: usize) -> StoreBuilder {
        StoreBuilder::new(database_path, dimensions)
    }

    #[must_use]
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Atomically applies nodes, edges, embeddings, and a new durable generation.
    ///
    /// # Errors
    ///
    /// Returns an error when the batch is invalid, references an unknown node, or
    /// cannot be committed to `SQLite`. A post-commit accelerator failure is reported
    /// through [`CommitReceipt::accelerator_ready`] instead of as a commit failure.
    pub fn apply_batch(&mut self, batch: &WriteBatch) -> Result<CommitReceipt> {
        self.require_writable()?;
        self.validate_batch(batch)?;
        if batch.nodes.is_empty() && batch.edges.is_empty() && batch.embeddings.is_empty() {
            return Err(Error::InvalidInput(
                "write batch must not be empty".to_string(),
            ));
        }

        let transaction = self.connection.transaction()?;
        for node in &batch.nodes {
            transaction.execute(
                "INSERT INTO nodes(id, repository_id, kind, name, content)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    repository_id = excluded.repository_id,
                    kind = excluded.kind,
                    name = excluded.name,
                    content = excluded.content",
                params![
                    node.id,
                    node.repository_id,
                    node.kind,
                    node.name,
                    node.content
                ],
            )?;
        }
        for embedding in &batch.embeddings {
            let key = node_key(&transaction, &embedding.node_id)?;
            transaction.execute(
                "INSERT INTO embeddings(node_key, vector) VALUES (?1, ?2)
                 ON CONFLICT(node_key) DO UPDATE SET vector = excluded.vector",
                params![key, encode_vector(&embedding.vector)],
            )?;
        }
        for edge in &batch.edges {
            let source_key = node_key(&transaction, &edge.source_id)?;
            let target_key = node_key(&transaction, &edge.target_id)?;
            let source_repository = node_repository(&transaction, source_key)?;
            let target_repository = node_repository(&transaction, target_key)?;
            if source_repository != target_repository {
                return Err(Error::InvalidInput(format!(
                    "edge crosses repository boundary: {} -> {}",
                    edge.source_id, edge.target_id
                )));
            }
            transaction.execute(
                "INSERT INTO edges(source_key, target_key, relationship)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(source_key, relationship, target_key) DO NOTHING",
                params![source_key, target_key, edge.relationship],
            )?;
        }
        let generation: u64 = transaction.query_row(
            "UPDATE store_metadata SET generation = generation + 1
             WHERE singleton = 1 RETURNING generation",
            [],
            |row| row.get(0),
        )?;
        transaction.commit()?;

        let rebuilt = VectorAccelerator::rebuild(
            &self.connection,
            self.vector_path.clone(),
            self.dimensions,
            generation,
            self.durability,
        )
        .ok();
        let accelerator_ready = rebuilt.is_some();
        *self
            .accelerator
            .lock()
            .map_err(|_| Error::Storage("vector accelerator lock is poisoned".to_string()))? =
            rebuilt;

        Ok(CommitReceipt {
            generation,
            accelerator_ready,
        })
    }

    /// Searches canonical embeddings using the requested retrieval strategy.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid query, unavailable requested accelerator,
    /// poisoned internal synchronization, or storage failure.
    pub fn search(
        &self,
        query: &[f32],
        limit: usize,
        filter: SearchFilter<'_>,
        requested_backend: SearchBackend,
    ) -> Result<SearchResults> {
        self.validate_vector(query, "query")?;
        if limit == 0 {
            return Err(Error::InvalidInput(
                "search limit must be positive".to_string(),
            ));
        }

        let candidate_count = self.candidate_count(filter)?;
        let backend = match requested_backend {
            SearchBackend::Adaptive if candidate_count <= self.exact_search_threshold => {
                SearchBackend::Exact
            }
            SearchBackend::Adaptive => SearchBackend::Usearch,
            explicit => explicit,
        };
        let hits = match backend {
            SearchBackend::Exact => exact_search(&self.connection, query, limit, filter)?,
            SearchBackend::Usearch => {
                let generation = self.generation()?;
                let mut accelerator = self.accelerator.lock().map_err(|_| {
                    Error::Storage("vector accelerator lock is poisoned".to_string())
                })?;
                let needs_rebuild = accelerator.as_ref().is_none_or(|index| {
                    !index.is_current(generation)
                        || index.dimensions() != self.dimensions
                        || index.path() != self.vector_path
                });
                if needs_rebuild {
                    if self.read_only {
                        return Err(Error::AcceleratorUnavailable(
                            "read-only stores cannot rebuild a missing or stale sidecar"
                                .to_string(),
                        ));
                    }
                    *accelerator = Some(VectorAccelerator::rebuild(
                        &self.connection,
                        self.vector_path.clone(),
                        self.dimensions,
                        generation,
                        self.durability,
                    )?);
                }
                accelerator
                    .as_ref()
                    .ok_or_else(|| {
                        Error::Storage("vector accelerator failed to initialize".to_string())
                    })?
                    .search(&self.connection, query, limit, filter)?
            }
            SearchBackend::Adaptive => unreachable!("adaptive backend resolves before execution"),
        };

        Ok(SearchResults { backend, hits })
    }

    /// Traverses a bounded, repository-local graph neighborhood.
    ///
    /// # Errors
    ///
    /// Returns an error when the start node does not exist or a storage query fails.
    pub fn traverse(
        &self,
        start_id: &str,
        direction: Direction,
        relationship: Option<&str>,
        max_hops: usize,
        max_results: usize,
    ) -> Result<Vec<Node>> {
        if max_hops == 0 || max_results == 0 {
            return Ok(Vec::new());
        }
        let start_key = node_key(&self.connection, start_id)?;
        let repository_id: i64 = self.connection.query_row(
            "SELECT repository_id FROM nodes WHERE key = ?1",
            [start_key],
            |row| row.get(0),
        )?;
        let mut visited = HashSet::from([start_key]);
        let mut frontier = VecDeque::from([(start_key, 0_usize)]);
        let mut nodes = Vec::new();

        while let Some((key, depth)) = frontier.pop_front() {
            if depth == max_hops {
                continue;
            }
            let mut neighbors = self.neighbor_keys(key, direction, relationship)?;
            neighbors.sort_unstable();
            for neighbor_key in neighbors {
                if !visited.insert(neighbor_key) {
                    continue;
                }
                let node = load_node(&self.connection, neighbor_key)?;
                if node.repository_id != repository_id {
                    continue;
                }
                frontier.push_back((neighbor_key, depth + 1));
                nodes.push(node);
                if nodes.len() == max_results {
                    return Ok(nodes);
                }
            }
        }

        Ok(nodes)
    }

    /// Returns the current durable store generation.
    ///
    /// # Errors
    ///
    /// Returns an error when metadata cannot be read.
    pub fn generation(&self) -> Result<u64> {
        Ok(self.connection.query_row(
            "SELECT generation FROM store_metadata WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?)
    }

    /// Returns the number of durable nodes.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot execute the count.
    pub fn node_count(&self) -> Result<usize> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))?)
    }

    /// Returns the number of durable canonical embeddings.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot execute the count.
    pub fn embedding_count(&self) -> Result<usize> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?)
    }

    /// Returns operational counts and accelerator readiness.
    ///
    /// # Errors
    ///
    /// Returns an error when durable metadata or counts cannot be read.
    pub fn stats(&self) -> Result<StoreStats> {
        let generation = self.generation()?;
        let edge_count = self
            .connection
            .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;
        let accelerator_ready = self
            .accelerator
            .lock()
            .map_err(|_| Error::Storage("vector accelerator lock is poisoned".to_string()))?
            .as_ref()
            .is_some_and(|accelerator| accelerator.is_current(generation));

        Ok(StoreStats {
            generation,
            node_count: self.node_count()?,
            edge_count,
            embedding_count: self.embedding_count()?,
            accelerator_ready,
            read_only: self.read_only,
        })
    }

    fn require_writable(&self) -> Result<()> {
        if self.read_only {
            Err(Error::ReadOnly)
        } else {
            Ok(())
        }
    }

    fn validate_batch(&self, batch: &WriteBatch) -> Result<()> {
        let mut node_ids = HashSet::new();
        for node in &batch.nodes {
            if node.id.trim().is_empty() || !node_ids.insert(node.id.as_str()) {
                return Err(Error::InvalidInput(
                    "node IDs must be non-empty and unique within a batch".to_string(),
                ));
            }
        }
        let mut embedding_ids = HashSet::new();
        for embedding in &batch.embeddings {
            if !embedding_ids.insert(embedding.node_id.as_str()) {
                return Err(Error::InvalidInput(
                    "embedding node IDs must be unique within a batch".to_string(),
                ));
            }
            self.validate_vector(&embedding.vector, "embedding")?;
        }
        Ok(())
    }

    fn validate_vector(&self, vector: &[f32], label: &str) -> Result<()> {
        if vector.len() != self.dimensions {
            return Err(Error::InvalidInput(format!(
                "{label} dimension mismatch: expected {}, received {}",
                self.dimensions,
                vector.len()
            )));
        }
        if vector.iter().any(|coordinate| !coordinate.is_finite()) {
            return Err(Error::InvalidInput(format!(
                "{label} contains a non-finite coordinate"
            )));
        }
        Ok(())
    }

    fn candidate_count(&self, filter: SearchFilter<'_>) -> Result<usize> {
        Ok(self.connection.query_row(
            "SELECT COUNT(*) FROM nodes AS n
             JOIN embeddings AS e ON e.node_key = n.key
             WHERE (?1 IS NULL OR n.repository_id = ?1)
               AND (?2 IS NULL OR n.kind = ?2)",
            (filter.repository_id, filter.kind),
            |row| row.get(0),
        )?)
    }

    fn neighbor_keys(
        &self,
        key: i64,
        direction: Direction,
        relationship: Option<&str>,
    ) -> Result<Vec<i64>> {
        let mut keys = Vec::new();
        if matches!(direction, Direction::Outgoing | Direction::Both) {
            let mut statement = self.connection.prepare(
                "SELECT target_key FROM edges
                 WHERE source_key = ?1 AND (?2 IS NULL OR relationship = ?2)",
            )?;
            keys.extend(
                statement
                    .query_map((key, relationship), |row| row.get::<_, i64>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?,
            );
        }
        if matches!(direction, Direction::Incoming | Direction::Both) {
            let mut statement = self.connection.prepare(
                "SELECT source_key FROM edges
                 WHERE target_key = ?1 AND (?2 IS NULL OR relationship = ?2)",
            )?;
            keys.extend(
                statement
                    .query_map((key, relationship), |row| row.get::<_, i64>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?,
            );
        }
        Ok(keys)
    }
}

fn acquire_writer_lock(database_path: &Path) -> Result<File> {
    let lock_path = database_path.with_extension("tsg.lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    lock.try_lock_exclusive()
        .map_err(|_| Error::WriterLocked(lock_path.display().to_string()))?;
    Ok(lock)
}

fn initialize_schema(connection: &Connection, dimensions: usize) -> Result<()> {
    let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(Error::UnsupportedSchema {
            found: version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    connection.execute_batch(SCHEMA)?;
    let dimensions_i64 = i64::try_from(dimensions).map_err(|_| {
        Error::InvalidInput("embedding dimensions exceed SQLite integer range".to_string())
    })?;
    connection.execute(
        "INSERT OR IGNORE INTO store_metadata(singleton, dimensions, generation)
         VALUES (1, ?1, 0)",
        [dimensions_i64],
    )?;
    connection.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    Ok(())
}

fn validate_schema(connection: &Connection) -> Result<()> {
    let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version != CURRENT_SCHEMA_VERSION {
        return Err(Error::UnsupportedSchema {
            found: version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    Ok(())
}

fn node_key(connection: &Connection, node_id: &str) -> Result<i64> {
    connection
        .query_row("SELECT key FROM nodes WHERE id = ?1", [node_id], |row| {
            row.get(0)
        })
        .optional()?
        .ok_or_else(|| Error::InvalidInput(format!("unknown node ID: {node_id}")))
}

fn load_node(connection: &Connection, key: i64) -> Result<Node> {
    Ok(connection.query_row(
        "SELECT id, repository_id, kind, name, content FROM nodes WHERE key = ?1",
        [key],
        |row| {
            Ok(Node {
                id: row.get(0)?,
                repository_id: row.get(1)?,
                kind: row.get(2)?,
                name: row.get(3)?,
                content: row.get(4)?,
            })
        },
    )?)
}

fn node_repository(connection: &Connection, key: i64) -> Result<i64> {
    Ok(connection.query_row(
        "SELECT repository_id FROM nodes WHERE key = ?1",
        [key],
        |row| row.get(0),
    )?)
}
