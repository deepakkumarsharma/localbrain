use serde::Serialize;
use thiserror::Error;

use crate::graph::{GraphContext, GraphStore};
use crate::metadata::MetadataStore;
use crate::search::{hybrid_search, SearchError, SearchResult};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChatAnswer {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub graph_context: Vec<GraphContext>,
    pub provider: String,
}

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("search error: {0}")]
    Search(#[from] SearchError),
    #[error("graph error: {0}")]
    Graph(#[from] crate::graph::GraphError),
}

pub async fn ask_local(
    query: &str,
    metadata_store: &MetadataStore,
    graph_store: &GraphStore,
) -> Result<ChatAnswer, LlmError> {
    let results = hybrid_search(metadata_store, query, 6).await?;
    let citations = results.iter().map(citation_from_result).collect::<Vec<_>>();
    let graph_context = graph_context_for_results(graph_store, &results)?;
    let answer = compose_answer(query, &citations, &graph_context);

    Ok(ChatAnswer {
        answer,
        citations,
        graph_context,
        provider: "local-retrieval".to_string(),
    })
}

fn citation_from_result(result: &SearchResult) -> Citation {
    Citation {
        path: result.path.clone(),
        title: result.title.clone(),
        snippet: result.snippet.clone(),
        score: result.score,
    }
}

fn graph_context_for_results(
    graph_store: &GraphStore,
    results: &[SearchResult],
) -> Result<Vec<GraphContext>, crate::graph::GraphError> {
    let mut contexts = Vec::new();

    for result in results.iter().take(3) {
        let mut context = graph_store.get_graph_context(&result.path, 24)?;
        contexts.append(&mut context);
    }

    contexts.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.symbol.name.cmp(&right.symbol.name))
    });
    contexts.dedup_by(|left, right| {
        left.path == right.path
            && left.relation == right.relation
            && left.symbol.name == right.symbol.name
            && left.symbol.range.start_line == right.symbol.range.start_line
    });
    contexts.truncate(24);

    Ok(contexts)
}

fn compose_answer(query: &str, citations: &[Citation], graph_context: &[GraphContext]) -> String {
    if citations.is_empty() {
        return format!(
            "I could not find indexed evidence for \"{query}\". Rebuild the search index or index more files, then ask again."
        );
    }

    let mut lines = vec![
        format!("Local answer for \"{query}\":"),
        String::new(),
        "I found relevant indexed context in these files:".to_string(),
    ];

    for citation in citations.iter().take(3) {
        lines.push(format!(
            "- `{}` ({:.2}) - {}",
            citation.path,
            citation.score,
            truncate_snippet(&citation.snippet)
        ));
    }

    if !graph_context.is_empty() {
        lines.push(String::new());
        lines.push("Nearby graph symbols:".to_string());
        for context in graph_context.iter().take(6) {
            lines.push(format!(
                "- `{}` in `{}` at L{}",
                context.symbol.name, context.path, context.symbol.range.start_line
            ));
        }
    }

    lines.push(String::new());
    lines.push(
        "This is a retrieval-grounded local draft, not an LLM-generated explanation yet."
            .to_string(),
    );

    lines.join("\n")
}

fn truncate_snippet(snippet: &str) -> String {
    const MAX_CHARS: usize = 180;

    let trimmed = snippet.trim();
    if trimmed.chars().count() <= MAX_CHARS {
        return trimmed.to_string();
    }

    let mut truncated = trimmed.chars().take(MAX_CHARS).collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::compose_answer;

    #[test]
    fn empty_context_returns_no_evidence_answer() {
        let answer = compose_answer("router", &[], &[]);

        assert!(answer.contains("could not find indexed evidence"));
    }
}
