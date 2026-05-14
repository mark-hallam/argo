use std::fmt;

/// Errors returned while building or validating a skill graph.
#[derive(Debug, Clone, PartialEq)]
pub enum SkillBuildError {
    /// A node id was declared more than once.
    DuplicateNode(String),
    /// An edge id was declared more than once.
    DuplicateEdge(String),
    /// A variable was declared more than once.
    DuplicateVariable(String),
    /// The entry id is empty or does not refer to a node.
    MissingEntry(String),
    /// An edge references a missing source or target node.
    MissingEdgeNode(String),
    /// A confidence annotation is outside `0.0..=1.0`.
    InvalidConfidence(String),
    /// A branch has no arms.
    EmptyBranch(String),
    /// A loop has no body nodes.
    EmptyLoop(String),
    /// A try/catch node has an empty try or catch body.
    EmptyTryCatch(String),
}

impl fmt::Display for SkillBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode(id) => write!(f, "duplicate node id: {id}"),
            Self::DuplicateEdge(id) => write!(f, "duplicate edge id: {id}"),
            Self::DuplicateVariable(name) => write!(f, "duplicate variable: {name}"),
            Self::MissingEntry(id) => write!(f, "entry node not found: {id}"),
            Self::MissingEdgeNode(id) => write!(f, "edge {id} references a missing node"),
            Self::InvalidConfidence(id) => write!(f, "node {id} confidence must be 0..1"),
            Self::EmptyBranch(id) => write!(f, "branch node {id} needs at least one arm"),
            Self::EmptyLoop(id) => write!(f, "loop node {id} needs a body"),
            Self::EmptyTryCatch(id) => {
                write!(f, "try-catch node {id} needs try and catch bodies")
            }
        }
    }
}

impl std::error::Error for SkillBuildError {}
