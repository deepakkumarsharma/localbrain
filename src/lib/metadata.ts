import { invoke } from '@tauri-apps/api/core';

export type FileChangeStatus = 'new' | 'changed' | 'unchanged' | 'deleted' | 'error';

export interface FileMetadata {
  path: string;
  language: string | null;
  sizeBytes: number;
  modifiedAt: string | null;
  contentHash: string;
  lastIndexedAt: string | null;
  status: FileChangeStatus;
}

export async function recordFileMetadata(path: string) {
  return invoke<FileMetadata>('record_file_metadata', { path });
}

export async function getFileMetadata(path: string) {
  return invoke<FileMetadata | null>('get_file_metadata', { path });
}

export async function checkFileChanged(path: string) {
  return invoke<FileChangeStatus>('check_file_changed', { path });
}
