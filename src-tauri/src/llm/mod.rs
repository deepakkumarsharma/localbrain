use serde::Serialize;
use thiserror::Error;

use crate::graph::{GraphContext, GraphStore};
use crate::metadata::MetadataStore;
use crate::search::{
    document_for_path, hybrid_search, indexed_documents, project_overview_chunks, IndexedDocument,
    SearchError, SearchResult,
};
use crate::settings::SettingsStore;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryIntent {
    FileList,
    ProjectOverview,
    General,
}

pub async fn ask_local(
    query: &str,
    metadata_store: &MetadataStore,
    graph_store: &GraphStore,
) -> Result<ChatAnswer, LlmError> {
    const MIN_EVIDENCE_SCORE: f32 = 0.20;
    const MIN_EVIDENCE_TEXT_SCORE: f32 = 0.04;
    const MIN_EVIDENCE_VECTOR_SCORE: f32 = 0.20;
    const MIN_GENERATION_SCORE: f32 = 0.45;
    const MIN_TEXT_SCORE: f32 = 0.08;
    const MIN_VECTOR_SCORE: f32 = 0.55;

    let settings = app
        .state::<SettingsStore>()
        .get()
        .map_err(LlmError::Generation)?;
    let intent = classify_intent(query);
    let documents = indexed_documents(metadata_store, 250).await?;
    let search_limit = match intent {
        QueryIntent::ProjectOverview => 14,
        QueryIntent::FileList | QueryIntent::General => 8,
    };
    let mut results = if intent == QueryIntent::ProjectOverview {
        let mut overview_results = project_overview_chunks(metadata_store, 10).await?;
        overview_results.extend(hybrid_search(metadata_store, query, search_limit).await?);
        dedupe_results(&mut overview_results);
        overview_results
    } else {
        hybrid_search(metadata_store, query, search_limit).await?
    };
    if let Some(path) = active_path.filter(|path| query_targets_path(query, path)) {
        if let Some(focused_result) = document_for_path(metadata_store, path, query).await? {
            results.retain(|result| result.path != focused_result.path);
            results.insert(0, focused_result);
        }
    }

    let relevant_results = results
        .into_iter()
        .filter(|result| {
            is_relevant_result(
                result,
                MIN_EVIDENCE_SCORE,
                MIN_EVIDENCE_TEXT_SCORE,
                MIN_EVIDENCE_VECTOR_SCORE,
            )
        })
        .collect::<Vec<_>>();
    let citations = relevant_results
        .iter()
        .map(citation_from_result)
        .collect::<Vec<_>>();
    let graph_context = graph_context_for_results(graph_store, &relevant_results);

    if intent == QueryIntent::FileList {
        return Ok(ChatAnswer {
            answer: format_file_list_answer(&documents),
            citations: citations_for_documents(&documents, 12),
            graph_context,
            provider: "local-index".to_string(),
        });
    }

    let mut used_llm = false;
    let answer = if settings.local_model_path.is_some() {
        let state = app.state::<local::LocalLlmState>();
        if state.is_running()
            && query_is_meaningful(query)
            && (intent == QueryIntent::ProjectOverview
                || has_relevant_results(
                    &relevant_results,
                    MIN_GENERATION_SCORE,
                    MIN_TEXT_SCORE,
                    MIN_VECTOR_SCORE,
                ))
        {
            let prompt = build_prompt(query, intent, &documents, &citations, &graph_context);
            match local::generate_with_llama(&prompt, state.server_port).await {
                Ok(generated) => {
                    used_llm = true;
                    format_answer(
                        query,
                        intent,
                        Some(&generated),
                        &documents,
                        &citations,
                        &graph_context,
                    )
                }
                Err(_) => {
                    format_answer(query, intent, None, &documents, &citations, &graph_context)
                }
            }
        } else {
            format_answer(query, intent, None, &documents, &citations, &graph_context)
        }
    } else {
        format_answer(query, intent, None, &documents, &citations, &graph_context)
    };

    Ok(ChatAnswer {
        answer,
        citations,
        graph_context,
        provider: "local-retrieval".to_string(),
    })
}

fn query_targets_path(query: &str, path: &str) -> bool {
    let query = query.to_lowercase();
    let path = path.to_lowercase();
    let file_name = path.rsplit('/').next().unwrap_or(&path);

    query.contains(&path)
        || query.contains(file_name)
        || query.contains("this file")
        || query.contains("current file")
        || query.contains("selected file")
}

fn is_relevant_result(
    result: &SearchResult,
    min_score: f32,
    min_text_score: f32,
    min_vector_score: f32,
) -> bool {
    result.score >= min_score
        && (result.text_score >= min_text_score || result.vector_score >= min_vector_score)
}

fn query_is_meaningful(query: &str) -> bool {
    let trimmed = query.trim();
    if trimmed.len() < 3 {
        return false;
    }

    let alpha_count = trimmed
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .count();
    if alpha_count < 3 {
        return false;
    }

    let token_count = trimmed
        .split_whitespace()
        .filter(|token| {
            token
                .chars()
                .any(|character| character.is_ascii_alphabetic())
        })
        .count();

    token_count >= 2 || trimmed.len() >= 8
}

fn has_relevant_results(
    results: &[SearchResult],
    min_score: f32,
    min_text_score: f32,
    min_vector_score: f32,
) -> bool {
    results.first().is_some_and(|result| {
        result.score >= min_score
            && (result.text_score >= min_text_score || result.vector_score >= min_vector_score)
    })
}

fn classify_intent(query: &str) -> QueryIntent {
    let query = query.to_lowercase();
    let file_list_terms = [
        "give me the files",
        "list files",
        "show files",
        "what files",
        "which files",
        "files in this project",
        "file list",
    ];
    if file_list_terms.iter().any(|term| query.contains(term)) {
        return QueryIntent::FileList;
    }

    let overview_terms = [
        "what this project",
        "what is this project",
        "what does this project",
        "what is the project",
        "what does the project",
        "project doing",
        "explain this project",
        "summarize this project",
        "overview of this project",
    ];
    if overview_terms.iter().any(|term| query.contains(term)) {
        return QueryIntent::ProjectOverview;
    }

    QueryIntent::General
}

fn build_prompt(
    query: &str,
    intent: QueryIntent,
    documents: &[IndexedDocument],
    citations: &[Citation],
    graph_context: &[GraphContext],
) -> String {
    let mut context_text = String::new();

    if intent == QueryIntent::ProjectOverview {
        context_text.push_str("### Indexed File Map\n");
        for document in documents.iter().take(80) {
            context_text.push_str(&format!("- {} ({})\n", document.path, document.kind));
        }
        context_text.push('\n');
    }

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

    let answer_shape = match intent {
        QueryIntent::ProjectOverview => {
            "Answer with: what the project does, main folders/files, runtime flow, and likely extension points."
        }
        QueryIntent::FileList => "Return a concise grouped file list.",
        QueryIntent::General => {
            "Answer the user's exact question. Include file paths and line/symbol evidence when useful."
        }
    };

    format!(
        "System: You are Local Brain's codebase assistant. Use ONLY the indexed context below. Be direct, specific, and grounded in file paths. If the context is insufficient, say exactly what is missing from the current index. {}\n\n{}\nQuestion: {}\n\nAssistant:",
        answer_shape, context_text, query
    )
}

fn citations_for_documents(documents: &[IndexedDocument], limit: usize) -> Vec<Citation> {
    documents
        .iter()
        .take(limit)
        .map(|document| Citation {
            path: document.path.clone(),
            title: document.title.clone(),
            snippet: format!("Indexed {} file", document.kind),
            start_line: None,
            end_line: None,
            score: 1.0,
        })
        .collect()
}

fn format_file_list_answer(documents: &[IndexedDocument]) -> String {
    if documents.is_empty() {
        return "I don't know from current index. No indexed files are available yet.".to_string();
    }

    let mut lines = vec![
        "## Indexed Files".to_string(),
        format!("Found {} indexed files.", documents.len()),
        String::new(),
    ];

    for document in documents.iter().take(120) {
        lines.push(format!("- `{}`", document.path));
    }
    if documents.len() > 120 {
        lines.push(format!("- ...and {} more", documents.len() - 120));
    }

    lines.join("\n")
}

fn citation_from_result(result: &SearchResult) -> Citation {
    Citation {
        path: result.path.clone(),
        title: result.title.clone(),
        snippet: result.snippet.clone(),
        start_line: result.start_line,
        end_line: result.end_line,
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

fn dedupe_results(results: &mut Vec<SearchResult>) {
    let mut seen = std::collections::HashSet::new();
    results.retain(|result| seen.insert((result.path.clone(), result.chunk_id.clone())));
}

fn format_answer(
    query: &str,
    intent: QueryIntent,
    llm_output: Option<&str>,
    documents: &[IndexedDocument],
    citations: &[Citation],
    graph_context: &[GraphContext],
) -> String {
    let summary = llm_output
        .map(clean_summary)
        .filter(|text| is_grounded_summary(text, citations))
        .unwrap_or_else(|| fallback_summary(query, citations));

    if intent == QueryIntent::ProjectOverview && llm_output.is_none() {
        return format_project_overview_answer(query, documents, citations, graph_context);
    }

    let mut lines = vec!["## Answer".to_string(), summary];
    lines.push(String::new());
    lines.push("## Evidence".to_string());

    if citations.is_empty() {
        lines.push("- No indexed evidence found yet. Rebuild the search index.".to_string());
    } else {
        for citation in citations.iter().take(4) {
            lines.push(format!(
                "- `{}` ({:.2}): {}",
                citation_label(citation),
                citation.score,
                truncate_snippet(&citation.snippet)
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Files".to_string());
    if citations.is_empty() {
        lines.push("- No file path available from current index.".to_string());
    } else {
        for citation in citations.iter().take(5) {
            lines.push(format!("- `{}`", citation_label(citation)));
        }
    }

    lines.push(String::new());
    lines.push("## Symbols".to_string());
    if graph_context.is_empty() {
        lines.push("- Usage graph context is not available for these files yet.".to_string());
    } else {
        for context in graph_context.iter().take(6) {
            lines.push(format!(
                "- `{}` in `{}` at L{}",
                context.symbol.name, context.path, context.symbol.range.start_line
            ));
        }
    }
    lines.join("\n")
}

fn format_project_overview_answer(
    query: &str,
    documents: &[IndexedDocument],
    citations: &[Citation],
    graph_context: &[GraphContext],
) -> String {
    if documents.is_empty() && citations.is_empty() {
        return fallback_summary(query, citations);
    }

    let mut lines = vec![
        "## Project Overview".to_string(),
        format!(
            "This workspace has {} indexed files. Based on the current index, it appears to be a code project organized around these main areas:",
            documents.len()
        ),
        String::new(),
        "## Main Files".to_string(),
    ];

    for document in documents.iter().take(12) {
        lines.push(format!("- `{}`", document.path));
    }

    lines.push(String::new());
    lines.push("## Likely Responsibilities".to_string());
    if citations.is_empty() {
        lines.push("- The search index has file paths, but not enough text evidence for a detailed summary yet.".to_string());
    } else {
        for citation in citations.iter().take(5) {
            lines.push(format!(
                "- `{}`: {}",
                citation_label(citation),
                truncate_snippet(&citation.snippet)
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Code Map".to_string());
    if graph_context.is_empty() {
        lines.push("- Symbol graph context is not available for the top matches yet.".to_string());
    } else {
        for context in graph_context.iter().take(8) {
            lines.push(format!(
                "- `{}` in `{}` at L{}",
                context.symbol.name, context.path, context.symbol.range.start_line
            ));
        }
    }

    lines.join("\n")
}

fn clean_summary(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            let stripped = line.trim_start_matches('#').trim();
            !matches!(
                stripped.to_ascii_lowercase().as_str(),
                "assistant:" | "summary"
            )
        })
        .take(40)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(2400)
        .collect()
}

fn is_grounded_summary(summary: &str, citations: &[Citation]) -> bool {
    if summary.trim().is_empty() || citations.is_empty() {
        return false;
    }
    let lower = summary.to_lowercase();

    citations.iter().take(3).any(|citation| {
        let path = citation.path.to_lowercase();
        let file_name = path.rsplit('/').next().unwrap_or(path.as_str());
        lower.contains(file_name)
            || citation
                .snippet
                .split_whitespace()
                .take(10)
                .any(|token| token.len() > 4 && lower.contains(&token.to_lowercase()))
    })
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

fn citation_label(citation: &Citation) -> String {
    match (citation.start_line, citation.end_line) {
        (Some(start), Some(end)) if start == end => format!("{}:L{}", citation.path, start),
        (Some(start), Some(end)) => format!("{}:L{}-L{}", citation.path, start, end),
        _ => citation.path.clone(),
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
    use super::{
        classify_intent, format_answer, format_file_list_answer, is_grounded_summary, Citation,
        IndexedDocument, QueryIntent,
    };

    #[test]
    fn empty_context_returns_no_evidence_answer() {
        let answer = format_answer("router", QueryIntent::General, None, &[], &[], &[]);

        assert!(answer.contains("could not find indexed evidence"));
    }

    #[test]
    fn detects_file_list_intent() {
        assert_eq!(classify_intent("give me the files"), QueryIntent::FileList);
        assert_eq!(
            classify_intent("what is this project doing?"),
            QueryIntent::ProjectOverview
        );
    }

    #[test]
    fn file_list_answer_returns_indexed_paths() {
        let documents = vec![IndexedDocument {
            path: "pageindex/page_index.py".to_string(),
            kind: "code".to_string(),
            title: "page_index.py".to_string(),
        }];

        let answer = format_file_list_answer(&documents);

        assert!(answer.contains("pageindex/page_index.py"));
    }

    #[test]
    fn summary_must_reference_citation_evidence() {
        let citations = vec![Citation {
            path: "src/indexer/mod.rs".to_string(),
            title: "mod.rs".to_string(),
            snippet: "fn index_path scans files and updates metadata".to_string(),
            start_line: Some(42),
            end_line: Some(64),
            score: 0.91,
        }];

        assert!(is_grounded_summary(
            "The behavior is in mod.rs where index_path scans files.",
            &citations
        ));
        assert!(!is_grounded_summary(
            "This is about conceptual architecture and implementation stages.",
            &citations
        ));
    }
}
