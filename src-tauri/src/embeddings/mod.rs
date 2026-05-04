use serde::Serialize;
use sha2::{Digest, Sha256};

pub const EMBEDDING_DIMENSIONS: usize = 64;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingSummary {
    pub path: String,
    pub dimensions: usize,
    pub magnitude: f32,
}

pub fn embed_text(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0_f32; EMBEDDING_DIMENSIONS];

    for token in tokenize(text) {
        let digest = Sha256::digest(token.as_bytes());
        let mut index_bytes = [0_u8; 8];
        index_bytes.copy_from_slice(&digest[..8]);
        let index = u64::from_le_bytes(index_bytes) as usize % EMBEDDING_DIMENSIONS;
        let sign = if digest[8] & 1 == 0 { 1.0 } else { -1.0 };
        vector[index] += sign;
    }

    normalize(&mut vector);
    vector
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum()
}

pub fn vector_magnitude(vector: &[f32]) -> f32 {
    vector.iter().map(|value| value * value).sum::<f32>().sqrt()
}

fn normalize(vector: &mut [f32]) {
    let magnitude = vector_magnitude(vector);
    if magnitude == 0.0 {
        return;
    }

    for value in vector {
        *value /= magnitude;
    }
}

fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| token.len() > 1)
        .map(|token| token.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{cosine_similarity, embed_text, vector_magnitude, EMBEDDING_DIMENSIONS};

    #[test]
    fn embeds_text_into_stable_normalized_vector() {
        let first = embed_text("React component renders local code intelligence");
        let second = embed_text("React component renders local code intelligence");

        assert_eq!(first, second);
        assert_eq!(first.len(), EMBEDDING_DIMENSIONS);
        assert!((vector_magnitude(&first) - 1.0).abs() < 0.001);
    }

    #[test]
    fn similar_text_scores_higher_than_unrelated_text() {
        let query = embed_text("file watcher index");
        let related = embed_text("watcher indexes changed files");
        let unrelated = embed_text("presentation slide theme");

        assert!(cosine_similarity(&query, &related) > cosine_similarity(&query, &unrelated));
    }
}
