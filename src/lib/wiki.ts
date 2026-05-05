import { invoke } from '@tauri-apps/api/core';

export interface WikiSummary {
  root: string;
  outputDir: string;
  pagesWritten: number;
  indexPath: string;
  errors: string[];
}

export async function generate_wiki(path: string) {
  return invoke<WikiSummary>('generate_wiki', { path });
}

export async function getWikiContent(path: string) {
  return invoke<string | null>('get_wiki_content', { path });
}
