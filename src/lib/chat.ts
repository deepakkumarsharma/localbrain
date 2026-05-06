import { invoke } from '@tauri-apps/api/core';
import type { GraphContext } from './graph';

export interface Citation {
  path: string;
  title: string;
  snippet: string;
  score: number;
}

export interface ChatAnswer {
  answer: string;
  citations: Citation[];
  graphContext: GraphContext[];
  provider: string;
}

export type ChatRole = 'user' | 'assistant';

export interface ChatMessage {
  id: string;
  role: ChatRole;
  content: string;
  citations: Citation[];
  createdAt: number;
  status: 'complete' | 'pending' | 'error';
}

export async function askLocal(query: string, activePath?: string) {
  return invoke<ChatAnswer>('ask_local', { query, activePath });
}
