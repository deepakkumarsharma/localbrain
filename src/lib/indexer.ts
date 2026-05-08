import { invoke } from '@tauri-apps/api/core';
import type { GraphIngestSummary } from './graph';
import type { FileChangeStatus, FileMetadata } from './metadata';

export interface IndexRunSummary {
  id: number;
  startedAt: string;
  finishedAt: string | null;
  filesSeen: number;
  filesChanged: number;
  status: string;
}

export interface IndexFileSummary {
  path: string;
  status: FileChangeStatus;
  skipped: boolean;
  metadata: FileMetadata | null;
  graph: GraphIngestSummary | null;
}

export interface IndexPathSummary {
  path: string;
  filesSeen: number;
  filesChanged: number;
  filesSkipped: number;
  filesDeleted: number;
  errors: string[];
  run: IndexRunSummary | null;
  files: IndexFileSummary[];
}

export interface IndexProgressEvent {
  runId: number | null;
  phase: string;
  filesSeen: number;
  filesTotal: number;
  filesChanged: number;
  filesSkipped: number;
  filesDeleted: number;
  errors: number;
  currentPath: string | null;
}

export async function indexFile(path: string) {
  return invoke<IndexFileSummary>('index_file', { path });
}

export async function indexPath(path: string, runId?: number) {
  return invoke<IndexPathSummary>('index_path', { path, runId });
}

export async function setWorkspaceRoot(path: string) {
  return invoke<string>('set_workspace_root', { path });
}

export async function resolveProjectRoot(path: string) {
  return invoke<string>('resolve_project_root', { path });
}

export async function getIndexStatus() {
  return invoke<IndexRunSummary | null>('get_index_status');
}
