import { invoke } from '@tauri-apps/api/core';
import type { CodeSymbol } from './parser';

export interface GraphIngestSummary {
  filePath: string;
  language: string;
  symbolCount: number;
  containsCount: number;
  symbolNames: string[];
}

export interface GraphContext {
  path: string;
  relation: string;
  symbol: CodeSymbol;
}

export interface GraphViewNode {
  id: string;
  label: string;
  kind: string;
}

export interface GraphViewEdge {
  id: string;
  source: string;
  target: string;
  label: string;
}

export interface GraphViewData {
  nodes: GraphViewNode[];
  edges: GraphViewEdge[];
}

export async function indexFileToGraph(path: string) {
  return invoke<GraphIngestSummary>('index_file_to_graph', { path });
}

export async function getGraphSymbols(path: string) {
  return invoke<CodeSymbol[]>('get_graph_symbols', { path });
}

export async function getGraphContext(target: string, limit = 24) {
  return invoke<GraphContext[]>('get_graph_context', { target, limit });
}

export async function getGraphView(path: string, limit = 40) {
  return invoke<GraphViewData>('get_graph_view', { path, limit });
}
