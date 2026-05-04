import { invoke } from '@tauri-apps/api/core';

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
  return invoke<SearchResult[]>('search_code', { query, limit });
}

export async function hybridSearch(query: string, limit = 10) {
  return invoke<SearchResult[]>('hybrid_search', { query, limit });
}
