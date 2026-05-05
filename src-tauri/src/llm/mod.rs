pub mod local;

use serde::Serialize;
use tauri::Manager;
use thiserror::Error;

use crate::graph::{GraphContext, GraphStore};
use crate::metadata::MetadataStore;
use crate::search::{hybrid_search, SearchError, SearchResult};
use crate::settings::SettingsStore;

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
    #[error("generation error: {0}")]
    Generation(String),
}

pub async fn ask_local(
    query: &str,
    metadata_store: &MetadataStore,
    graph_store: &GraphStore,
    app: &tauri::AppHandle,
) -> Result<ChatAnswer, LlmError> {
    let settings = app
        .state::<SettingsStore>()
        .get()
        .map_err(LlmError::Generation)?;
    let results = hybrid_search(metadata_store, query, 6).await?;
    let citations = results.iter().map(citation_from_result).collect::<Vec<_>>();
    let graph_context = graph_context_for_results(graph_store, &results);

    let answer = if settings.local_model_path.is_some() {
        let state = app.state::<local::LocalLlmState>();
        if state.is_running() {
            let prompt = build_prompt(query, &citations, &graph_context);
            let generated = local::generate_with_llama(&prompt, state.server_port)
                .await
                .map_err(LlmError::Generation)?;
            format_answer(query, Some(&generated), &citations, &graph_context)
        } else {
            format_answer(query, None, &citations, &graph_context)
        }
    } else {
        format_answer(query, None, &citations, &graph_context)
    };

    let is_llm =
        settings.local_model_path.is_some() && app.state::<local::LocalLlmState>().is_running();

    Ok(ChatAnswer {
        answer,
        citations,
        graph_context,
        provider: if is_llm {
            "llama-cpp".to_string()
        } else {
            "local-retrieval".to_string()
        },
    })
}

fn build_prompt(query: &str, citations: &[Citation], graph_context: &[GraphContext]) -> String {
    let mut context_text = String::new();

    context_text.push_str("### Code Context\n");
    for (i, citation) in citations.iter().enumerate() {
        context_text.push_str(&format!(
            "File {}: {}\nContent:\n{}\n\n",
            i + 1,
            citation.path,
            citation.snippet
        ));
    }

    if !graph_context.is_empty() {
        context_text.push_str("### Related Symbols\n");
        for context in graph_context {
            context_text.push_str(&format!(
                "- Symbol '{}' in '{}' (Relation: {})\n",
                context.symbol.name, context.path, context.relation
            ));
        }
    }

    format!(
        "System: You are a precise codebase assistant. Use ONLY provided context. Be concise, no repetition, no apologies, no self-references. If unknown, say exactly 'I don't know from current index.'\nReturn exactly these sections:\nSummary\n- bullet\n- bullet\nWhere the file path is\nHow this file is being used\nHow it is connected to other files\n\n{}\nQuestion: {}\n\nAssistant:",
        context_text, query
    )
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
) -> Vec<GraphContext> {
    let mut contexts = Vec::new();

    for result in results.iter().take(3) {
        if let Ok(mut context) = graph_store.get_graph_context(&result.path, 24) {
            contexts.append(&mut context);
        }
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

    contexts
}

fn format_answer(
    query: &str,
    llm_output: Option<&str>,
    citations: &[Citation],
    graph_context: &[GraphContext],
) -> String {
    let summary = llm_output
        .map(clean_summary)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| fallback_summary(query, citations));

    let mut lines = vec!["## Summary".to_string(), summary];
    lines.push(String::new());
    lines.push("## Key Points".to_string());

    if citations.is_empty() {
        lines.push("- No indexed evidence found yet. Rebuild the search index.".to_string());
    } else {
        for citation in citations.iter().take(4) {
            lines.push(format!(
                "- `{}` ({:.2}): {}",
                citation.path,
                citation.score,
                truncate_snippet(&citation.snippet)
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Where the file path is".to_string());
    if citations.is_empty() {
        lines.push("- No file path available from current index.".to_string());
    } else {
        for citation in citations.iter().take(5) {
            lines.push(format!("- `{}`", citation.path));
        }
    }

    lines.push(String::new());
    lines.push("## How this file is being used".to_string());
    if graph_context.is_empty() {
        lines.push("- Usage graph context is not available for these files yet.".to_string());
    } else {
        for context in graph_context.iter().take(6) {
            lines.push(format!(
                "- `{}` appears in `{}` at L{} ({})",
                context.symbol.name,
                context.path,
                context.symbol.range.start_line,
                context.relation
            ));
        }
    }

    lines.push(String::new());
    lines.push("## How it is connected to other files".to_string());
    if graph_context.is_empty() {
        lines.push(
            "- Cross-file graph connections are not available from current context.".to_string(),
        );
    } else {
        let primary = citations.first().map(|c| c.path.as_str()).unwrap_or("");
        let mut added = 0usize;
        for context in graph_context.iter() {
            if context.path != primary {
                lines.push(format!(
                    "- Connected symbol `{}` in `{}` ({})",
                    context.symbol.name, context.path, context.relation
                ));
                added += 1;
            }
            if added >= 6 {
                break;
            }
        }
        if added == 0 {
            lines.push("- Connections are currently within the same file scope.".to_string());
        }
    }

    lines.join("\n")
}

fn clean_summary(value: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    for line in value.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let normalized = line.to_lowercase();
        if seen.insert(normalized) {
            return truncate_snippet(line);
        }
    }
    String::new()
}

fn fallback_summary(query: &str, citations: &[Citation]) -> String {
    if let Some(first) = citations.first() {
        format!(
            "Best indexed match for \"{query}\" is `{}` with relevant code context.",
            first.path
        )
    } else {
        format!(
            "I don't know from current index for \"{query}\". Rebuild search index to include more files."
        )
    }
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
    use super::format_answer;

    #[test]
    fn empty_context_returns_no_evidence_answer() {
        let answer = format_answer("router", None, &[], &[]);

        assert!(answer.contains("I don't know from current index"));
    }
}
