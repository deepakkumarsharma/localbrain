import { invoke } from '@tauri-apps/api/core';

export type LlmProvider = 'local' | 'anthropic' | 'gemini' | 'openAi';

export interface ProviderSettings {
  provider: LlmProvider;
  cloudEnabled: boolean;
}

export async function getProviderSettings() {
  return invoke<ProviderSettings>('get_provider_settings');
}

export async function setProvider(provider: LlmProvider, cloudEnabled: boolean) {
  return invoke<ProviderSettings>('set_provider', { provider, cloudEnabled });
}
