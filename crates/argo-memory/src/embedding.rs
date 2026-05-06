//! Embedding provider adapters for indexed memory retrieval.
use argo_types::{ArgoError, ArgoResult};
use sha2::{Digest, Sha256};

pub(crate) const EMBEDDING_DIMS: usize = 64;
pub(crate) const VOYAGE_3_MODEL: &str = "voyage-3";

/// Embedding provider used by the indexed layer.
pub trait EmbeddingProvider: Send + Sync {
    /// Provider model identifier.
    fn model(&self) -> &'static str;

    /// Embed text into a fixed-size vector.
    fn embed(&self, text: &str) -> ArgoResult<Vec<f32>>;
}

/// Voyage-3 embedding adapter.
///
/// The runtime provider hook is represented by this trait implementation.
/// Until external credentials are injected, it produces deterministic local
/// vectors with the Voyage-3 model id so indexing and tenant isolation can be
/// tested without network calls.
#[derive(Debug, Default, Clone, Copy)]
pub struct Voyage3EmbeddingProvider;

impl EmbeddingProvider for Voyage3EmbeddingProvider {
    fn model(&self) -> &'static str {
        VOYAGE_3_MODEL
    }

    fn embed(&self, text: &str) -> ArgoResult<Vec<f32>> {
        Ok(hash_embedding(text))
    }
}

pub(crate) fn hash_embedding(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0_f32; EMBEDDING_DIMS];
    for token in text.split(|c: char| !c.is_alphanumeric()) {
        if token.is_empty() {
            continue;
        }
        let mut hasher = Sha256::new();
        hasher.update(token.to_ascii_lowercase().as_bytes());
        let digest = hasher.finalize();
        let index = usize::from(digest[0]) % EMBEDDING_DIMS;
        let sign = if digest[1] & 1 == 0 { 1.0 } else { -1.0 };
        vector[index] += sign;
    }
    normalize(&mut vector);
    vector
}

pub(crate) fn cosine(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| a * b)
        .sum::<f32>()
}

pub(crate) fn vector_to_json(vector: &[f32]) -> ArgoResult<String> {
    serde_json::to_string(vector)
        .map_err(|e| ArgoError::Provider(format!("memory vector serialize: {e}")))
}

pub(crate) fn vector_from_json(json: &str) -> Option<Vec<f32>> {
    serde_json::from_str::<Vec<f32>>(json).ok()
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector {
            *value /= norm;
        }
    }
}
