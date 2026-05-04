import { invoke } from '@tauri-apps/api/core';

const MAX_SEARCH_LIMIT = 100;

export interface SearchIndexSummary {
  root: string;
  documentsIndexed: number;
  embeddingsIndexed: number;
  errors: string[];
}

export interface SearchResult {
  path: string;
  kind: string;
  title: string;
  snippet: string;
  textScore: number;
  vectorScore: number;
  score: number;
}

export async function rebuildSearchIndex(path: string) {
  return invoke<SearchIndexSummary>('rebuild_search_index', { path });
}

export async function searchCode(query: string, limit = 10) {
  return invoke<SearchResult[]>('search_code', { query, limit: sanitizeLimit(limit) });
}

export async function hybridSearch(query: string, limit = 10) {
  return invoke<SearchResult[]>('hybrid_search', { query, limit: sanitizeLimit(limit) });
}

function sanitizeLimit(limit: number) {
  if (!Number.isFinite(limit)) {
    return 1;
  }

  return Math.min(MAX_SEARCH_LIMIT, Math.max(1, Math.trunc(limit)));
}
