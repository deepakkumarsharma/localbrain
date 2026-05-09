import { invoke } from '@tauri-apps/api/core';

export type LlmProvider = 'local' | 'anthropic' | 'gemini' | 'openAi';

export interface ProviderSettings {
  provider: LlmProvider;
  cloudEnabled: boolean;
  localModelPath: string | null;
  embeddingModelPath: string | null;
  lastProjectPath: string | null;
}

export async function getProviderSettings() {
  return invoke<ProviderSettings>('get_provider_settings');
}

export async function setProvider(provider: LlmProvider, cloudEnabled: boolean) {
  return invoke<ProviderSettings>('set_provider', { provider, cloudEnabled });
}

export async function setLocalModelPath(path: string | null) {
  return invoke<ProviderSettings>('set_local_model_path', { path });
}

export async function setEmbeddingModelPath(path: string | null) {
  return invoke<ProviderSettings>('set_embedding_model_path', { path });
}

export async function setLastProjectPath(path: string | null) {
  return invoke<ProviderSettings>('set_last_project_path', { path });
}

export async function startLocalLlm() {
  return invoke<void>('start_local_llm');
}

export async function stopLocalLlm() {
  return invoke<void>('stop_local_llm');
}

export async function getLocalLlmStatus() {
  return invoke<boolean>('get_local_llm_status');
}
