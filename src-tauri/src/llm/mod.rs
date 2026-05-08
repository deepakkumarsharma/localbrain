pub mod local;

use serde::Serialize;
use tauri::Manager;
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
    #[error("generation error: {0}")]
    Generation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryIntent {
    FileList,
    ProjectOverview,
    General,
}

pub async fn ask_local(
    query: &str,
    active_path: Option<&str>,
    metadata_store: &MetadataStore,
    graph_store: &GraphStore,
    app: &tauri::AppHandle,
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
    let mut focused_result: Option<SearchResult> = None;
    if let Some(path) = active_path {
        focused_result = document_for_path(metadata_store, path, query).await?;
        if let Some(ref focused) = focused_result {
            results.retain(|result| result.path != focused.path);
            if intent != QueryIntent::ProjectOverview
                && (query_targets_path(query, path) || results.is_empty())
            {
                results.insert(0, focused.clone());
            } else if intent != QueryIntent::ProjectOverview {
                results.push(focused.clone());
            }
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
    let mut citations = relevant_results
        .iter()
        .map(citation_from_result)
        .collect::<Vec<_>>();
    if citations.is_empty() {
        if let Some(ref focused) = focused_result {
            citations.push(citation_from_result(focused));
        }
    }
    rank_citations(&mut citations, active_path);
    let graph_context = graph_context_for_results(graph_store, &relevant_results)?;

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
        provider: if used_llm {
            "llama-cpp".to_string()
        } else {
            "local-retrieval".to_string()
        },
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
        let line_span = match (citation.start_line, citation.end_line) {
            (Some(start), Some(end)) if start == end => format!("{start}-{end}"),
            (Some(start), Some(end)) => format!("{start}-{end}"),
            _ => "unknown-lines".to_string(),
        };
        context_text.push_str(&format!(
            "File {} {}:{}\nContent:\n{}\n\n",
            i + 1,
            citation.path,
            line_span,
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
            "Return a professional architecture walkthrough with sections: Executive Summary, System Structure, Runtime Flow, Key Modules, Risks, and Next Steps."
        }
        QueryIntent::FileList => "Return a concise grouped file list.",
        QueryIntent::General => {
            "Answer as a principal software engineer mentoring a teammate. Be detailed and explicit. Use sections: Direct Answer, Technical Breakdown, Evidence, Risks/Trade-offs, and Suggested Next Steps."
        }
    };

    format!(
        "System: You are Local Brain's codebase assistant. Use ONLY the indexed context below. Be precise, technical, and mentorship-oriented. Never invent files, symbols, behavior, or line numbers. Always cite file paths with line numbers when available. Prefer markdown sections, bullet points, and short code-grounded explanations. If context is insufficient, explicitly say what is missing from current index. {}\n\n{}\nQuestion: {}\n\nAssistant:",
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

fn rank_citations(citations: &mut [Citation], active_path: Option<&str>) {
    citations.sort_by(|left, right| {
        citation_priority(right, active_path)
            .cmp(&citation_priority(left, active_path))
            .then_with(|| {
                right
                    .score
                    .partial_cmp(&left.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
}

fn citation_priority(citation: &Citation, active_path: Option<&str>) -> i32 {
    let mut priority = 0;
    if let Some(path) = active_path {
        if citation.path == path {
            priority += 100;
        }
    }
    if !citation.path.starts_with("docs/wiki/") {
        priority += 20;
    }
    if citation.start_line.is_some() && citation.end_line.is_some() {
        priority += 10;
    }
    priority
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

    let file_name = citations
        .first()
        .map(|c| file_name_from_path(&c.path))
        .unwrap_or_else(|| "unknown".to_string());

    let mut lines = vec![
        format!("# 📂 Module: `{}`", file_name),
        format!("> **Summary:** {}", summary),
        "---".to_string(),
    ];

    if citations.is_empty() {
        lines.push("### 🔍 System Note".to_string());
        lines.push("No indexed evidence found yet.".to_string());
        lines.push("No executable logic found. This file likely contains constants, types, or documentation.".to_string());
    } else {
        lines.push("## 🔄 Data Lifecycle".to_string());
        lines.push("Evidence-backed flow from indexed snippets:".to_string());
        for citation in citations.iter().take(3) {
            lines.push(format!(
                "- `{}`: {}",
                citation_label(citation),
                truncate_snippet(&citation.snippet)
            ));
        }

        lines.push(String::new());
        lines.push("## 🧱 Structural Blueprint".to_string());
        lines.push("| Symbol | Type | Role | Interaction |".to_string());
        lines.push("| :--- | :--- | :--- | :--- |".to_string());
        for citation in citations.iter().take(5) {
            let meta = analyze_deep(citation);
            lines.push(format!(
                "| `{}` | {} | {} | {} |",
                meta.name, meta.kind, meta.responsibility, meta.interaction
            ));
        }

        lines.push(String::new());
        lines.push("## 🧠 Logic Breakdown".to_string());
        for citation in citations.iter().take(3) {
            lines.push(format!("### 🔹 {}", citation.title));
            lines.push(format!(
                "**Intent:** {}",
                infer_intent_explanation(citation)
            ));
            lines.push(format!("**Evidence:** `{}`", citation_label(citation)));
            lines.push(citation.snippet.clone());
            lines.push(format!(
                "> **Translation:** {}",
                infer_plain_english_logic(citation)
            ));
            lines.push(String::new());
        }

        lines.push("## 🛠 Developer Cheat Sheet".to_string());
        for citation in citations.iter().take(4) {
            lines.push(format!(
                "- Start with `{}` for safe edits in this area.",
                citation_label(citation)
            ));
        }
    }
    lines.join("\n")
}

struct DeepCitationMeta {
    name: String,
    kind: String,
    responsibility: String,
    interaction: String,
}

fn analyze_deep(citation: &Citation) -> DeepCitationMeta {
    let lower = citation.snippet.to_lowercase();
    let kind = if lower.contains("struct ") {
        "Struct"
    } else if lower.contains("enum ") {
        "Enum"
    } else if lower.contains("trait ") {
        "Trait"
    } else if lower.contains("fn ") {
        "Function"
    } else {
        "Code Block"
    };

    let interaction = if lower.contains("select ")
        || lower.contains("insert ")
        || lower.contains("update ")
        || lower.contains("delete ")
    {
        "Database"
    } else if lower.contains("http")
        || lower.contains("request")
        || lower.contains("response")
        || lower.contains("api")
    {
        "API/Network"
    } else {
        "In-process"
    };

    DeepCitationMeta {
        name: citation.title.clone(),
        kind: kind.to_string(),
        responsibility: infer_intent_explanation(citation),
        interaction: interaction.to_string(),
    }
}

fn infer_intent_explanation(citation: &Citation) -> String {
    if citation.title.trim().is_empty() {
        "Provides supporting module logic.".to_string()
    } else {
        format!(
            "Implements `{}` behavior with source-backed logic.",
            citation.title
        )
    }
}

fn infer_plain_english_logic(citation: &Citation) -> String {
    let snippet = citation.snippet.trim();
    if snippet.is_empty() {
        return "No snippet available in current index.".to_string();
    }
    format!(
        "This block handles `{}` and contributes to the module workflow.",
        citation.title
    )
}

fn file_name_from_path(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
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
            let lower = line.to_ascii_lowercase();
            !(lower.starts_with("import ")
                || lower.starts_with("export ")
                || lower.starts_with("from ")
                || lower.contains("```")
                || lower.starts_with("</")
                || lower.starts_with("<"))
        })
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
    if summary.lines().count() > 32 {
        return false;
    }
    let lower = summary.to_lowercase();

    citations.iter().take(3).any(|citation| {
        let path = citation.path.to_lowercase();
        let file_name = path.rsplit('/').next().unwrap_or(path.as_str());
        if lower.contains(file_name) {
            return true;
        }

        let mut distinct_matches = std::collections::BTreeSet::new();
        for token in citation.snippet.split_whitespace().take(10) {
            if token.len() <= 4 {
                continue;
            }
            let normalized = token
                .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '/')
                .to_lowercase();
            if normalized.len() > 4 && lower.contains(&normalized) {
                distinct_matches.insert(normalized);
            }
        }
        distinct_matches.len() >= 2
    })
}

fn fallback_summary(query: &str, citations: &[Citation]) -> String {
    if let Some(first) = citations.first() {
        let mut lines = vec![
            format!(
                "For \"{query}\", the best indexed context is in `{}`.",
                first.path
            ),
            "What I can confirm from local index:".to_string(),
        ];
        for citation in citations.iter().take(2) {
            lines.push(format!(
                "- `{}`: {}",
                citation_label(citation),
                truncate_snippet(&citation.snippet)
            ));
        }
        lines.push("If this is not the file you intended, select the target file in Explorer and ask again so I can prioritize that context.".to_string());
        lines.join("\n")
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

        assert!(answer.contains("No indexed evidence found yet"));
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
