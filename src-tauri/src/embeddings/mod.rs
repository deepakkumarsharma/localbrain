use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::{Mutex, OnceLock, RwLock};
use tokenizers::Tokenizer;
use tract_onnx::prelude::*;

pub const EMBEDDING_DIMENSIONS: usize = 64;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingSummary {
    pub path: String,
    pub dimensions: usize,
    pub magnitude: f32,
}

static EMBEDDING_MODEL_PATH: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static ONNX_BACKEND_CACHE: OnceLock<Mutex<Option<CachedOnnxBackend>>> = OnceLock::new();

struct CachedOnnxBackend {
    model_path: String,
    tokenizer: Tokenizer,
    model: TypedRunnableModel<TypedModel>,
}

fn embedding_model_path_cell() -> &'static RwLock<Option<String>> {
    EMBEDDING_MODEL_PATH.get_or_init(|| RwLock::new(None))
}

fn onnx_backend_cache_cell() -> &'static Mutex<Option<CachedOnnxBackend>> {
    ONNX_BACKEND_CACHE.get_or_init(|| Mutex::new(None))
}

pub fn set_embedding_model_path(path: Option<String>) {
    if let Ok(mut guard) = embedding_model_path_cell().write() {
        *guard = path;
    }
}

pub fn embed_text(text: &str) -> Vec<f32> {
    if let Ok(guard) = embedding_model_path_cell().read() {
        if let Some(path) = guard.as_ref() {
            if Path::new(path).exists() {
                if let Ok(vector) = try_embed_with_onnx(text, path) {
                    return vector;
                }
            }
        }
    }

    embed_text_deterministic(text)
}

fn embed_text_deterministic(text: &str) -> Vec<f32> {
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

fn try_embed_with_onnx(text: &str, model_path: &str) -> Result<Vec<f32>, String> {
    const MAX_TOKENS: usize = 256;

    let mut cache_guard = onnx_backend_cache_cell()
        .lock()
        .map_err(|_| "onnx backend cache lock poisoned".to_string())?;
    let needs_reload = cache_guard
        .as_ref()
        .map(|cached| cached.model_path != model_path)
        .unwrap_or(true);

    if needs_reload {
        let tokenizer_path = Path::new(model_path)
            .parent()
            .map(|parent| parent.join("tokenizer.json"))
            .ok_or_else(|| "invalid embedding model path".to_string())?;
        if !tokenizer_path.exists() {
            return Err(format!(
                "missing tokenizer.json next to embedding model: {}",
                tokenizer_path.display()
            ));
        }

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|error| format!("failed to load tokenizer: {error}"))?;
        let model = tract_onnx::onnx()
            .model_for_path(model_path)
            .map_err(|error| format!("failed to load ONNX model: {error}"))?
            .into_optimized()
            .map_err(|error| format!("failed to optimize ONNX model: {error}"))?
            .into_runnable()
            .map_err(|error| format!("failed to prepare ONNX model: {error}"))?;
        *cache_guard = Some(CachedOnnxBackend {
            model_path: model_path.to_string(),
            tokenizer,
            model,
        });
    }

    let cached = cache_guard
        .as_mut()
        .ok_or_else(|| "onnx backend cache unavailable".to_string())?;
    let encoding = cached
        .tokenizer
        .encode(text, true)
        .map_err(|error| format!("tokenization failed: {error}"))?;

    let token_count = encoding.get_ids().len().clamp(1, MAX_TOKENS);
    let mut input_ids = vec![0_i64; token_count];
    let mut attention_mask = vec![0_i64; token_count];
    let token_type_ids = vec![0_i64; token_count];

    for index in 0..token_count {
        input_ids[index] = i64::from(encoding.get_ids()[index]);
        attention_mask[index] = i64::from(encoding.get_attention_mask()[index]);
    }

    let outputs = cached
        .model
        .run(tvec!(
            Tensor::from_shape(&[1, token_count], &input_ids)
                .map_err(|error| format!("input_ids tensor error: {error}"))?
                .into(),
            Tensor::from_shape(&[1, token_count], &attention_mask)
                .map_err(|error| format!("attention_mask tensor error: {error}"))?
                .into(),
            Tensor::from_shape(&[1, token_count], &token_type_ids)
                .map_err(|error| format!("token_type_ids tensor error: {error}"))?
                .into(),
        ))
        .map_err(|error| format!("ONNX inference failed: {error}"))?;

    let output_tensor = outputs
        .first()
        .ok_or_else(|| "ONNX model returned no outputs".to_string())?;
    let output = output_tensor
        .to_array_view::<f32>()
        .map_err(|error| format!("failed to read ONNX output tensor: {error}"))?;
    if output.ndim() != 3 {
        return Err(format!(
            "unexpected ONNX output shape rank: expected 3, got {}",
            output.ndim()
        ));
    }
    let hidden = *output
        .shape()
        .get(2)
        .ok_or_else(|| "missing hidden dimension in ONNX output".to_string())?;
    if hidden == 0 {
        return Err("ONNX output hidden dimension is zero".to_string());
    }

    let mut pooled = vec![0.0_f32; hidden];
    let mut valid_tokens = 0.0_f32;
    for token_index in 0..token_count.min(*output.shape().get(1).unwrap_or(&0)) {
        if attention_mask[token_index] == 0 {
            continue;
        }
        valid_tokens += 1.0;
        for dim in 0..hidden {
            pooled[dim] += output[[0, token_index, dim]];
        }
    }
    if valid_tokens == 0.0 {
        return Err("no valid tokens to pool from ONNX output".to_string());
    }
    for value in &mut pooled {
        *value /= valid_tokens;
    }
    normalize(&mut pooled);
    Ok(pooled)
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() {
        return 0.0;
    }
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
        .flat_map(split_identifier)
        .filter(|token| token.len() > 1)
        .map(|token| token.to_lowercase())
}

fn split_identifier(token: &str) -> Vec<String> {
    if token.is_empty() {
        return Vec::new();
    }

    let mut parts = Vec::new();
    let mut current = String::new();
    let chars = token.chars().collect::<Vec<_>>();

    for (index, ch) in chars.iter().enumerate() {
        let previous = if index > 0 {
            Some(chars[index - 1])
        } else {
            None
        };
        let next = chars.get(index + 1).copied();

        let boundary_before_upper = previous
            .map(|prev| prev.is_ascii_lowercase() && ch.is_ascii_uppercase())
            .unwrap_or(false);
        let boundary_in_acronym = previous
            .map(|prev| prev.is_ascii_uppercase() && ch.is_ascii_uppercase())
            .unwrap_or(false)
            && next.map(|n| n.is_ascii_lowercase()).unwrap_or(false);

        if (boundary_before_upper || boundary_in_acronym) && !current.is_empty() {
            parts.push(current.clone());
            current.clear();
        }

        current.push(*ch);
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts.push(token.to_string());
    parts
}

#[cfg(test)]
mod tests {
    use super::{
        cosine_similarity, embed_text, set_embedding_model_path, vector_magnitude,
        EMBEDDING_DIMENSIONS,
    };

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

    #[test]
    fn camel_case_identifiers_are_tokenized_for_semantic_recall() {
        let query = embed_text("active source path");
        let code = embed_text("activeSourcePath");
        let unrelated = embed_text("chart rendering palette");

        assert!(cosine_similarity(&query, &code) > cosine_similarity(&query, &unrelated));
    }

    #[test]
    fn defaults_to_deterministic_embedding_without_model_path() {
        set_embedding_model_path(None);
        let vector = embed_text("local deterministic embedding");
        assert_eq!(vector.len(), EMBEDDING_DIMENSIONS);
    }

    #[test]
    fn falls_back_to_deterministic_when_model_path_missing() {
        set_embedding_model_path(Some("/tmp/does-not-exist/model.onnx".to_string()));
        let vector = embed_text("feature graph relationship");
        assert_eq!(vector.len(), EMBEDDING_DIMENSIONS);
    }
}
