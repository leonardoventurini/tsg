use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use usearch::ffi::{IndexOptions, MetricKind, ScalarKind};

use crate::error::{Error, Result};
use crate::types::{Node, SearchFilter, SearchHit};

pub(crate) struct VectorAccelerator {
    index: usearch::Index,
    path: PathBuf,
    dimensions: usize,
    generation: u64,
}

impl VectorAccelerator {
    pub(crate) fn open_or_rebuild(
        connection: &Connection,
        path: PathBuf,
        dimensions: usize,
        generation: u64,
    ) -> Result<Self> {
        if sidecar_generation(&path) == Some(generation) && path.exists() {
            let index = new_index(dimensions)?;
            if index.load(path.to_string_lossy().as_ref()).is_ok() {
                return Ok(Self {
                    index,
                    path,
                    dimensions,
                    generation,
                });
            }
        }

        Self::rebuild(connection, path, dimensions, generation)
    }

    pub(crate) fn rebuild(
        connection: &Connection,
        path: PathBuf,
        dimensions: usize,
        generation: u64,
    ) -> Result<Self> {
        let index = new_index(dimensions)?;
        let vectors = load_vectors(connection, SearchFilter::default())?;
        index
            .reserve(vectors.len().max(1))
            .map_err(|error| Error::Storage(format!("reserve USearch capacity: {error}")))?;

        for (key, _, vector) in &vectors {
            index
                .add(*key, vector)
                .map_err(|error| Error::Storage(format!("add vector to USearch: {error}")))?;
        }

        persist_index(&index, &path, generation)?;

        Ok(Self {
            index,
            path,
            dimensions,
            generation,
        })
    }

    pub(crate) fn search(
        &self,
        connection: &Connection,
        query: &[f32],
        limit: usize,
        filter: SearchFilter<'_>,
    ) -> Result<Vec<SearchHit>> {
        let candidates = load_candidate_nodes(connection, filter)?;
        if candidates.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        // Searching the full eligible corpus guarantees correct post-filtering.
        // A future backend can push the allowlist into the ANN implementation.
        let search_limit = if filter == SearchFilter::default() {
            limit.min(candidates.len())
        } else {
            self.index.size()
        };
        let result = self
            .index
            .search(query, search_limit)
            .map_err(|error| Error::Storage(format!("search USearch index: {error}")))?;

        let mut hits = Vec::with_capacity(limit);
        for (key, distance) in result.keys.into_iter().zip(result.distances) {
            if let Some(node) = candidates.get(&key) {
                hits.push(SearchHit {
                    node: node.clone(),
                    distance,
                });
                if hits.len() == limit {
                    break;
                }
            }
        }
        sort_hits(&mut hits);

        Ok(hits)
    }

    pub(crate) fn is_current(&self, generation: u64) -> bool {
        self.generation == generation
    }

    pub(crate) fn dimensions(&self) -> usize {
        self.dimensions
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

pub(crate) fn exact_search(
    connection: &Connection,
    query: &[f32],
    limit: usize,
    filter: SearchFilter<'_>,
) -> Result<Vec<SearchHit>> {
    let query_norm = norm(query);
    if query_norm == 0.0 {
        return Err(Error::InvalidInput(
            "query vector must have a non-zero norm".to_string(),
        ));
    }

    let vectors = load_vectors(connection, filter)?;
    let mut hits = Vec::with_capacity(vectors.len());
    for (_, node, vector) in vectors {
        let vector_norm = norm(&vector);
        if vector_norm == 0.0 {
            continue;
        }
        let similarity = query
            .iter()
            .zip(&vector)
            .map(|(left, right)| left * right)
            .sum::<f32>()
            / (query_norm * vector_norm);
        hits.push(SearchHit {
            node,
            distance: 1.0 - similarity,
        });
    }
    sort_hits(&mut hits);
    hits.truncate(limit);

    Ok(hits)
}

pub(crate) fn encode_vector(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(vector));
    for coordinate in vector {
        bytes.extend_from_slice(&coordinate.to_le_bytes());
    }
    bytes
}

fn decode_vector(bytes: &[u8]) -> Result<Vec<f32>> {
    if bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(Error::Storage(
            "stored vector byte length is invalid".to_string(),
        ));
    }

    Ok(bytes
        .chunks_exact(std::mem::size_of::<f32>())
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

fn load_vectors(
    connection: &Connection,
    filter: SearchFilter<'_>,
) -> Result<Vec<(u64, Node, Vec<f32>)>> {
    let nodes = load_candidate_nodes(connection, filter)?;
    let eligible: HashSet<u64> = nodes.keys().copied().collect();
    let mut statement = connection.prepare("SELECT node_key, vector FROM embeddings")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
    })?;
    let mut vectors = Vec::new();
    for row in rows {
        let (signed_key, bytes) = row?;
        let key = u64::try_from(signed_key)
            .map_err(|_| Error::Storage("stored node key is negative".to_string()))?;
        if eligible.contains(&key) {
            let node = nodes
                .get(&key)
                .expect("eligible key originates from candidate map")
                .clone();
            vectors.push((key, node, decode_vector(&bytes)?));
        }
    }
    Ok(vectors)
}

fn load_candidate_nodes(
    connection: &Connection,
    filter: SearchFilter<'_>,
) -> Result<HashMap<u64, Node>> {
    let mut statement = connection.prepare(
        "SELECT key, id, repository_id, kind, name, content
         FROM nodes
         WHERE (?1 IS NULL OR repository_id = ?1)
           AND (?2 IS NULL OR kind = ?2)",
    )?;
    let rows = statement.query_map((filter.repository_id, filter.kind), |row| {
        Ok((
            row.get::<_, i64>(0)?,
            Node {
                id: row.get(1)?,
                repository_id: row.get(2)?,
                kind: row.get(3)?,
                name: row.get(4)?,
                content: row.get(5)?,
            },
        ))
    })?;

    let mut nodes = HashMap::new();
    for row in rows {
        let (signed_key, node) = row?;
        let key = u64::try_from(signed_key)
            .map_err(|_| Error::Storage("stored node key is negative".to_string()))?;
        nodes.insert(key, node);
    }
    Ok(nodes)
}

fn new_index(dimensions: usize) -> Result<usearch::Index> {
    let options = IndexOptions {
        dimensions,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        connectivity: 16,
        expansion_add: 128,
        expansion_search: 64,
        multi: false,
    };
    usearch::Index::new(&options)
        .map_err(|error| Error::Storage(format!("create USearch index: {error}")))
}

fn persist_index(index: &usearch::Index, path: &Path, generation: u64) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("usearch.tmp");
    index
        .save(temporary.to_string_lossy().as_ref())
        .map_err(|error| Error::Storage(format!("save USearch index: {error}")))?;
    File::open(&temporary)?.sync_all()?;
    std::fs::rename(&temporary, path)?;

    let generation_path = generation_path(path);
    let generation_temporary = generation_path.with_extension("generation.tmp");
    std::fs::write(&generation_temporary, generation.to_string())?;
    File::open(&generation_temporary)?.sync_all()?;
    std::fs::rename(generation_temporary, generation_path)?;
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

fn sidecar_generation(path: &Path) -> Option<u64> {
    std::fs::read_to_string(generation_path(path))
        .ok()?
        .parse()
        .ok()
}

fn generation_path(path: &Path) -> PathBuf {
    path.with_extension("usearch.generation")
}

fn norm(vector: &[f32]) -> f32 {
    vector
        .iter()
        .map(|coordinate| coordinate * coordinate)
        .sum::<f32>()
        .sqrt()
}

fn sort_hits(hits: &mut [SearchHit]) {
    hits.sort_by(|left, right| {
        left.distance
            .total_cmp(&right.distance)
            .then_with(|| left.node.id.cmp(&right.node.id))
    });
}
