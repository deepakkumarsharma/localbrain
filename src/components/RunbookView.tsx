import { useEffect, useMemo, useState } from 'react';
import { AlertTriangle, Loader2, Play, RefreshCw, Square, Terminal } from 'lucide-react';
import { getLocalLlmStatus } from '../lib/settings';
import {
  discoverRunbookCommands,
  getRunbookProcesses,
  listenRunbookLogs,
  listenRunbookProcessExited,
  listenRunbookProcessStarted,
  restartRunbookProcess,
  startRunbookProcess,
  stopRunbookProcess,
  type RunbookChecklist,
  type RunbookCommand,
  type RunbookLogEvent,
  type RunbookProcess,
} from '../lib/runbook';
import { useAppStore } from '../store/useAppStore';

export function RunbookView() {
  const { projectPath } = useAppStore();
  const [loading, setLoading] = useState(false);
  const [commands, setCommands] = useState<RunbookCommand[]>([]);
  const [processes, setProcesses] = useState<RunbookProcess[]>([]);
  const [logs, setLogs] = useState<RunbookLogEvent[]>([]);
  const [query, setQuery] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busyCommandId, setBusyCommandId] = useState<string | null>(null);
  const [checklist, setChecklist] = useState<RunbookChecklist>({
    projectSelected: false,
    dependenciesDetected: false,
    runtimeReady: false,
    localModelReady: false,
  });

  useEffect(() => {
    let mounted = true;
    let unlistenLogs: (() => void) | null = null;
    let unlistenStarted: (() => void) | null = null;
    let unlistenExited: (() => void) | null = null;

    async function bootstrap() {
      if (!projectPath) {
        setCommands([]);
        setProcesses([]);
        setLogs([]);
        setChecklist({
          projectSelected: false,
          dependenciesDetected: false,
          runtimeReady: false,
          localModelReady: false,
        });
        return;
      }

      setLoading(true);
      setError(null);
      try {
        const [discovery, currentProcesses, llmReady] = await Promise.all([
          discoverRunbookCommands(projectPath),
          getRunbookProcesses(),
          getLocalLlmStatus(),
        ]);
        if (!mounted) return;
        setCommands(discovery.commands);
        setProcesses(currentProcesses);
        setChecklist({
          ...discovery.checklist,
          localModelReady: llmReady,
        });
      } catch (err) {
        if (!mounted) return;
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        if (mounted) setLoading(false);
      }
    }

    void bootstrap();

    void listenRunbookLogs((event) => {
      if (!mounted) return;
      setLogs((existing) => [...existing.slice(-499), event]);
    }).then((unlisten) => {
      unlistenLogs = unlisten;
    });

    void listenRunbookProcessStarted((event) => {
      if (!mounted) return;
      setProcesses((existing) => {
        const filtered = existing.filter((item) => item.processId !== event.process.processId);
        return [event.process, ...filtered];
      });
    }).then((unlisten) => {
      unlistenStarted = unlisten;
    });

    void listenRunbookProcessExited((event) => {
      if (!mounted) return;
      setProcesses((existing) =>
        existing.map((item) => (item.processId === event.process.processId ? event.process : item)),
      );
    }).then((unlisten) => {
      unlistenExited = unlisten;
    });

    return () => {
      mounted = false;
      unlistenLogs?.();
      unlistenStarted?.();
      unlistenExited?.();
    };
  }, [projectPath]);

  const filteredLogs = useMemo(() => {
    const trimmed = query.trim().toLowerCase();
    if (!trimmed) return logs;
    return logs.filter(
      (item) =>
        item.line.toLowerCase().includes(trimmed) ||
        item.stream.toLowerCase().includes(trimmed) ||
        item.processId.toLowerCase().includes(trimmed),
    );
  }, [logs, query]);

  async function handleStart(command: RunbookCommand) {
    if (!projectPath) return;
    try {
      const confirmed = command.requiresConfirmation
        ? window.confirm(
            `Run "${command.command}" from ${command.cwd}?\n\nThis command is marked as requiring confirmation.`,
          )
        : true;
      if (!confirmed) {
        return;
      }
      setBusyCommandId(command.id);
      setError(null);
      await startRunbookProcess(projectPath, command, confirmed);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusyCommandId(null);
    }
  }

  async function handleStop(processId: string) {
    try {
      await stopRunbookProcess(processId);
      setProcesses((existing) =>
        existing.map((item) =>
          item.processId === processId
            ? { ...item, status: 'stopped', exitedAt: `${Date.now()}`, exitCode: 0 }
            : item,
        ),
      );
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      if (message.includes('runbook process not found')) {
        setProcesses((existing) => existing.filter((item) => item.processId !== processId));
        return;
      }
      setError(message);
    }
  }

  async function handleRestart(processId: string) {
    try {
      await restartRunbookProcess(processId);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      if (message.includes('runbook process not found')) {
        setProcesses((existing) => existing.filter((item) => item.processId !== processId));
        return;
      }
      setError(message);
    }
  }

  return (
    <div className="absolute inset-0 overflow-auto bg-app-background app-scrollbar">
      <div className="mx-auto max-w-[1600px] px-6 py-6">
        <div className="mb-4 rounded-xl border border-app-border bg-app-panel p-4">
          <div className="mb-2 flex items-center justify-between">
            <div className="text-[11px] font-black uppercase tracking-widest text-app-muted">
              Runbook V1.5
            </div>
            <div className="rounded-md border border-app-warning/40 bg-app-warning/15 px-2 py-1 text-[11px] font-bold text-app-warning">
              Guided execution only
            </div>
          </div>
          <div className="grid grid-cols-1 gap-3 md:grid-cols-4">
            <ChecklistCard
              label="Project selected"
              detail={projectPath ? 'Workspace loaded' : 'No project selected'}
              ok={checklist.projectSelected}
            />
            <ChecklistCard
              label="Dependencies detected"
              detail={
                commands.length > 0 ? `${commands.length} commands found` : 'No commands found'
              }
              ok={checklist.dependenciesDetected}
            />
            <ChecklistCard
              label="Runtime ready"
              detail="Local launch controls active"
              ok={checklist.runtimeReady}
            />
            <ChecklistCard
              label="Local model ready"
              detail={checklist.localModelReady ? 'LLM sidecar available' : 'LLM not running'}
              ok={checklist.localModelReady}
            />
          </div>
        </div>

        <div className="mb-4 rounded-xl border border-app-warning/40 bg-app-warning/10 p-3 text-sm text-app-text">
          <div className="mb-1 flex items-center gap-2 font-bold">
            <AlertTriangle className="h-4 w-4 text-app-warning" />
            Only detected or approved workspace commands can run.
          </div>
          <div className="text-[12px] text-app-muted">
            No arbitrary shell prompt is exposed in V1.5. Discovery is read-only and execution is
            workspace-scoped.
          </div>
        </div>

        {error ? (
          <div className="mb-4 rounded-xl border border-app-error/40 bg-app-error/10 p-3 text-sm text-app-error">
            {error}
          </div>
        ) : null}

        <div className="grid grid-cols-1 gap-4 xl:grid-cols-[1.2fr_0.8fr]">
          <section className="rounded-xl border border-app-border bg-app-panel p-4">
            <div className="mb-3 text-[11px] font-black uppercase tracking-widest text-app-muted">
              Command launcher
            </div>
            {loading ? (
              <div className="flex items-center gap-2 text-sm text-app-muted">
                <Loader2 className="h-4 w-4 animate-spin" />
                Discovering workspace commands...
              </div>
            ) : (
              <div className="space-y-2">
                {commands.map((command) => (
                  <div
                    key={command.id}
                    className="rounded-lg border border-app-border bg-app-background p-3"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <div className="min-w-0">
                        <div className="truncate text-sm font-bold text-app-text">
                          {command.name}
                        </div>
                        <div className="truncate font-mono text-[11px] text-app-muted">
                          {command.command}
                        </div>
                      </div>
                      <button
                        type="button"
                        className="inline-flex items-center gap-1 rounded-md border border-app-accent/40 bg-app-accent/15 px-2.5 py-1.5 text-[12px] font-bold text-app-accent disabled:opacity-50"
                        onClick={() => void handleStart(command)}
                        disabled={!projectPath || busyCommandId === command.id}
                      >
                        {busyCommandId === command.id ? (
                          <Loader2 className="h-3.5 w-3.5 animate-spin" />
                        ) : (
                          <Play className="h-3.5 w-3.5" />
                        )}
                        Start
                      </button>
                    </div>
                    <div className="mt-2 flex flex-wrap gap-2 text-[11px]">
                      <span className="rounded border border-app-border bg-app-panel px-1.5 py-0.5 text-app-muted">
                        {command.kind}
                      </span>
                      <span className="rounded border border-app-border bg-app-panel px-1.5 py-0.5 text-app-muted">
                        cwd: {command.cwd}
                      </span>
                      <span className="rounded border border-app-border bg-app-panel px-1.5 py-0.5 text-app-muted">
                        source: {command.source}
                      </span>
                      <span className="rounded border border-app-border bg-app-panel px-1.5 py-0.5 text-app-muted">
                        risk: {command.risk}
                      </span>
                    </div>
                  </div>
                ))}
                {commands.length === 0 && !loading ? (
                  <div className="rounded-lg border border-app-border bg-app-background p-3 text-sm text-app-muted">
                    No supported commands were detected in this workspace.
                  </div>
                ) : null}
              </div>
            )}
          </section>

          <section className="rounded-xl border border-app-border bg-app-panel p-4">
            <div className="mb-3 text-[11px] font-black uppercase tracking-widest text-app-muted">
              Process cards
            </div>
            <div className="space-y-2">
              {processes.map((process) => {
                const statusColor =
                  process.status === 'running'
                    ? 'text-app-success border-app-success/30 bg-app-success/10'
                    : process.exitCode && process.exitCode !== 0
                      ? 'text-app-error border-app-error/30 bg-app-error/10'
                      : 'text-app-warning border-app-warning/30 bg-app-warning/10';
                return (
                  <div
                    key={process.processId}
                    className="rounded-lg border border-app-border bg-app-background p-3"
                  >
                    <div className="mb-2 flex items-center justify-between gap-2">
                      <div className="min-w-0">
                        <div className="truncate text-sm font-bold text-app-text">
                          {process.name}
                        </div>
                        <div className="truncate font-mono text-[11px] text-app-muted">
                          {process.command}
                        </div>
                      </div>
                      <span
                        className={`rounded border px-2 py-0.5 text-[11px] font-bold ${statusColor}`}
                      >
                        {process.status}
                      </span>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <button
                        type="button"
                        className="inline-flex items-center gap-1 rounded-md border border-app-border bg-app-panel px-2 py-1 text-[12px] font-bold text-app-text"
                        onClick={() => void handleRestart(process.processId)}
                      >
                        <RefreshCw className="h-3.5 w-3.5" />
                        Restart
                      </button>
                      <button
                        type="button"
                        className="inline-flex items-center gap-1 rounded-md border border-app-border bg-app-panel px-2 py-1 text-[12px] font-bold text-app-text"
                        onClick={() => void handleStop(process.processId)}
                      >
                        <Square className="h-3.5 w-3.5" />
                        Stop
                      </button>
                    </div>
                  </div>
                );
              })}
              {processes.length === 0 ? (
                <div className="rounded-lg border border-app-border bg-app-background p-3 text-sm text-app-muted">
                  No processes started yet.
                </div>
              ) : null}
            </div>
          </section>
        </div>

        <section className="mt-4 rounded-xl border border-app-border bg-app-panel p-4">
          <div className="mb-3 flex items-center justify-between">
            <div className="flex items-center gap-2 text-[11px] font-black uppercase tracking-widest text-app-muted">
              <Terminal className="h-4 w-4" />
              Log stream
            </div>
            <input
              className="w-64 rounded-md border border-app-border bg-app-background px-2 py-1.5 text-[12px] text-app-text outline-none"
              type="text"
              placeholder="Search logs..."
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              aria-label="Search runbook logs"
            />
          </div>
          <div className="max-h-[320px] space-y-1 overflow-auto rounded-lg border border-app-border bg-app-background p-2 app-scrollbar">
            {filteredLogs.map((line, index) => (
              <div
                key={`${line.processId}-${line.ts}-${index}`}
                className="grid grid-cols-[110px_70px_minmax(0,1fr)] gap-2 border-b border-app-border/50 py-1 text-[11px] font-mono text-app-text"
              >
                <span className="truncate text-app-muted">{line.ts}</span>
                <span className={line.stream === 'stderr' ? 'text-app-error' : 'text-app-success'}>
                  {line.stream}
                </span>
                <span className="break-all">{line.line}</span>
              </div>
            ))}
            {filteredLogs.length === 0 ? (
              <div className="p-2 text-sm text-app-muted">No logs yet.</div>
            ) : null}
          </div>
        </section>
      </div>
    </div>
  );
}

function ChecklistCard({ label, detail, ok }: { label: string; detail: string; ok: boolean }) {
  return (
    <div className="rounded-lg border border-app-border bg-app-background px-3 py-2">
      <div className="mb-1 flex items-center justify-between">
        <div className="text-[11px] font-black uppercase tracking-wider text-app-muted">
          {label}
        </div>
        <div
          className={`h-2.5 w-2.5 rounded-full ${ok ? 'bg-app-success shadow-[0_0_8px_rgba(var(--color-app-success),0.45)]' : 'bg-app-warning shadow-[0_0_8px_rgba(var(--color-app-warning),0.45)]'}`}
        />
      </div>
      <div className="truncate text-[12px] text-app-text">{detail}</div>
    </div>
  );
}
