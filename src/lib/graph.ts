import { invoke } from '@tauri-apps/api/core';
import type { CodeSymbol } from './parser';

export interface GraphIngestSummary {
  filePath: string;
  language: string;
  symbolCount: number;
  containsCount: number;
  symbolNames: string[];
}

export async function indexFileToGraph(path: string) {
  return invoke<GraphIngestSummary>('index_file_to_graph', { path });
}

export async function getGraphSymbols(path: string) {
  return invoke<CodeSymbol[]>('get_graph_symbols', { path });
}
