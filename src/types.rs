#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub id: String,
    pub repository_id: i64,
    pub kind: String,
    pub name: String,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    pub source_id: String,
    pub target_id: String,
    pub relationship: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Embedding {
    pub node_id: String,
    pub vector: Vec<f32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WriteBatch {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub embeddings: Vec<Embedding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Outgoing,
    Incoming,
    Both,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchBackend {
    Exact,
    Usearch,
    Adaptive,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SearchFilter<'a> {
    pub repository_id: Option<i64>,
    pub kind: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub node: Node,
    pub distance: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchResults {
    pub backend: SearchBackend,
    pub hits: Vec<SearchHit>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitReceipt {
    pub generation: u64,
    pub accelerator_ready: bool,
}
