import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface RunbookCommand {
  id: string;
  name: string;
  command: string;
  cwd: string;
  source: string;
  kind: string;
  requiresConfirmation: boolean;
  risk: string;
}

export interface RunbookChecklist {
  projectSelected: boolean;
  dependenciesDetected: boolean;
  runtimeReady: boolean;
  localModelReady: boolean;
}

export interface RunbookDiscovery {
  workspaceRoot: string;
  commands: RunbookCommand[];
  checklist: RunbookChecklist;
}

export interface RunbookProcess {
  processId: string;
  commandId: string;
  kind: string;
  workspaceRoot: string;
  name: string;
  command: string;
  cwd: string;
  source: string;
  status: string;
  startedAt: string;
  exitedAt: string | null;
  exitCode: number | null;
}

export interface RunbookLogEvent {
  processId: string;
  stream: string;
  line: string;
  ts: string;
}

export interface RunbookProcessStartedEvent {
  process: RunbookProcess;
}

export interface RunbookProcessExitedEvent {
  process: RunbookProcess;
}

export async function discoverRunbookCommands(path: string) {
  return invoke<RunbookDiscovery>('discover_runbook_commands', { path });
}

export async function startRunbookProcess(
  workspaceRoot: string,
  command: RunbookCommand,
  confirmed = false,
) {
  return invoke<RunbookProcess>('start_runbook_process', {
    request: { workspaceRoot, command, confirmed },
  });
}

export async function stopRunbookProcess(processId: string) {
  return invoke<void>('stop_runbook_process', { processId });
}

export async function restartRunbookProcess(processId: string) {
  return invoke<RunbookProcess>('restart_runbook_process', { processId });
}

export async function getRunbookProcesses() {
  return invoke<RunbookProcess[]>('get_runbook_processes');
}

export async function listenRunbookLogs(
  handler: (event: RunbookLogEvent) => void,
): Promise<() => void> {
  const unlisten = await listen<RunbookLogEvent>('runbook-log', (event) => handler(event.payload));
  return unlisten;
}

export async function listenRunbookProcessStarted(
  handler: (event: RunbookProcessStartedEvent) => void,
): Promise<() => void> {
  const unlisten = await listen<RunbookProcessStartedEvent>('runbook-process-started', (event) =>
    handler(event.payload),
  );
  return unlisten;
}

export async function listenRunbookProcessExited(
  handler: (event: RunbookProcessExitedEvent) => void,
): Promise<() => void> {
  const unlisten = await listen<RunbookProcessExitedEvent>('runbook-process-exited', (event) =>
    handler(event.payload),
  );
  return unlisten;
}
