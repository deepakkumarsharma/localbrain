import { invoke } from '@tauri-apps/api/core';

export interface AgentApiStatus {
  running: boolean;
  bindAddress: string;
  localOnly: boolean;
}

export async function getAgentApiStatus() {
  return invoke<AgentApiStatus>('get_agent_api_status');
}

export async function startAgentApi() {
  return invoke<AgentApiStatus>('start_agent_api');
}

export async function stopAgentApi() {
  return invoke<AgentApiStatus>('stop_agent_api');
}
