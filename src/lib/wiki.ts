import { invoke } from '@tauri-apps/api/core';

export interface WikiSummary {
  root: string;
  outputDir: string;
  pagesWritten: number;
  indexPath: string;
  errors: string[];
}

export async function generateWiki(path: string) {
  return invoke<WikiSummary>('generate_wiki', { path });
}
