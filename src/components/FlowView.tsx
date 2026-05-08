import { TrendingUp } from 'lucide-react';
import { getLoadingProgress } from '../lib/progress';
import { useAppStore } from '../store/useAppStore';

interface FlowStep {
  title: string;
  detail: string;
  color: string;
  state: 'done' | 'waiting';
}

export function FlowView() {
  const {
    projectPath,
    isProjectLoading,
    projectStatus,
    indexProgress,
    activeSourcePath,
    indexPathSummary,
    wikiSummary,
    llmRunning,
    providerSettings,
    chatMessages,
  } = useAppStore();
  const loadingProgress = getLoadingProgress(indexProgress, projectStatus);

  if (!projectPath) {
    return (
      <div className="absolute inset-0 overflow-auto bg-app-background">
        <div className="mx-auto max-w-[720px] px-8 py-12">
          <h2 className="mb-4 flex items-center gap-2 text-lg font-semibold">
            <TrendingUp className="h-5 w-5 text-app-accent" aria-hidden="true" />
            Localbrain Request Flow
          </h2>
          <div className="rounded-xl border border-app-border bg-app-panel p-5 text-sm text-app-muted">
            Select a project folder from the left panel to start indexing and see the live flow.
          </div>
        </div>
      </div>
    );
  }

  if (isProjectLoading) {
    return (
      <div className="absolute inset-0 overflow-auto bg-app-background">
        <div className="mx-auto max-w-[720px] px-8 py-12">
          <h2 className="mb-4 flex items-center gap-2 text-lg font-semibold">
            <TrendingUp className="h-5 w-5 text-app-accent" aria-hidden="true" />
            Localbrain Request Flow
          </h2>
          <div className="rounded-xl border border-app-border bg-app-panel p-5 text-sm text-app-muted shadow-sm">
            <div className="mb-4 flex items-center justify-between gap-3">
              <div className="flex items-center gap-3">
                <span className="flow-status-dot h-2.5 w-2.5 rounded-full bg-app-accent" />
                <span className="flow-status-dot h-2.5 w-2.5 rounded-full bg-app-success [animation-delay:220ms]" />
                <span className="flow-status-dot h-2.5 w-2.5 rounded-full bg-app-warning [animation-delay:440ms]" />
                <span className="text-[12px] font-black uppercase tracking-widest text-app-muted">
                  Processing Workspace
                </span>
              </div>
              <span className="rounded-full border border-app-border bg-app-panelSoft px-2.5 py-1 font-mono text-xs font-semibold text-app-text">
                {loadingProgress.percentLabel}
              </span>
            </div>
            <div
              className="mb-3 h-2.5 overflow-hidden rounded-full bg-app-panelSoft"
              role="progressbar"
              aria-label="Flow load progress"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(loadingProgress.percent)}
              aria-valuetext={`${Math.round(loadingProgress.percent)}% loaded`}
            >
              <div
                className="flow-progress-fill h-full rounded-full bg-app-accent"
                style={{ width: `${loadingProgress.percent}%` }}
              />
            </div>
            <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
              <span>{loadingProgress.detail}</span>
              {loadingProgress.currentFile ? (
                <span className="truncate font-mono text-xs text-app-text">
                  · {loadingProgress.currentFile}
                </span>
              ) : null}
            </div>
          </div>
        </div>
      </div>
    );
  }

  const projectName = projectPath.split('/').pop() || projectPath;
  const hasIndex = Boolean(indexPathSummary?.filesSeen);
  const hasWiki = Boolean(wikiSummary?.pagesWritten);
  const hasActiveFile = Boolean(activeSourcePath);
  const hasModel = Boolean(providerSettings?.localModelPath) && llmRunning;
  const hasAsked = chatMessages.some((message) => message.role === 'user');
  const changed = indexPathSummary?.filesChanged ?? 0;
  const skipped = indexPathSummary?.filesSkipped ?? 0;
  const errors = indexPathSummary?.errors.length ?? 0;
  const wikiPages = wikiSummary?.pagesWritten ?? 0;

  const steps: FlowStep[] = [
    {
      title: 'Project selected',
      detail: projectName,
      color: 'rgb(var(--color-graph-feature))',
      state: 'done',
    },
    {
      title: 'Workspace indexed',
      detail: hasIndex
        ? `${indexPathSummary?.filesSeen ?? 0} files scanned`
        : 'Waiting for indexing results',
      color: 'rgb(var(--color-graph-component))',
      state: hasIndex ? 'done' : 'waiting',
    },
    {
      title: 'Wiki generated',
      detail: hasWiki ? `${wikiSummary?.pagesWritten ?? 0} pages written` : 'Wiki export pending',
      color: 'rgb(var(--color-graph-api))',
      state: hasWiki ? 'done' : 'waiting',
    },
    {
      title: 'Source focused',
      detail: hasActiveFile ? activeSourcePath : 'No active file selected yet',
      color: 'rgb(var(--color-graph-service))',
      state: hasActiveFile ? 'done' : 'waiting',
    },
    {
      title: 'Local ask ready',
      detail: hasModel
        ? hasAsked
          ? 'Questions asked and answered locally'
          : 'Model is ready for questions'
        : 'Select a model and start local server',
      color: 'rgb(var(--color-graph-model))',
      state: hasModel ? 'done' : 'waiting',
    },
  ];

  return (
    <div className="absolute inset-0 overflow-auto bg-app-background">
      <div className="mx-auto max-w-[1080px] px-8 py-12">
        <h2 className="mb-6 flex items-center gap-2 text-lg font-semibold">
          <TrendingUp className="h-5 w-5 text-app-accent" aria-hidden="true" />
          Localbrain Request Flow
        </h2>
        <div className="mb-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
          <FlowMetric
            label="Files Seen"
            value={String(indexPathSummary?.filesSeen ?? 0)}
            color="text-blue-400"
          />
          <FlowMetric label="Changed" value={String(changed)} color="text-emerald-400" />
          <FlowMetric label="Skipped" value={String(skipped)} color="text-amber-400" />
          <FlowMetric label="Wiki Pages" value={String(wikiPages)} color="text-violet-400" />
        </div>
        <div className="mb-6 rounded-xl border border-app-border bg-app-panel p-4">
          <div className="flex flex-wrap gap-2 text-[11px] font-black uppercase tracking-widest">
            <span className="rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-1 text-emerald-400">
              {hasModel ? 'LLM Ready' : 'LLM Waiting'}
            </span>
            <span className="rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-1 text-blue-400">
              {hasActiveFile ? 'File Focused' : 'No File Focus'}
            </span>
            <span className="rounded-full border border-violet-500/30 bg-violet-500/10 px-2 py-1 text-violet-400">
              {hasAsked ? 'Q&A Active' : 'Q&A Idle'}
            </span>
            <span
              className={`rounded-full border px-2 py-1 ${errors > 0 ? 'border-red-500/30 bg-red-500/10 text-red-400' : 'border-emerald-500/30 bg-emerald-500/10 text-emerald-400'}`}
            >
              {errors > 0 ? `${errors} Errors` : '0 Errors'}
            </span>
          </div>
        </div>
        <section className="min-w-0">
          <div className="relative">
            <div className="absolute bottom-6 left-[27px] top-6 w-px overflow-hidden rounded-full bg-app-border/70">
              <span className="flow-line-shot absolute left-0 top-0 h-10 w-full bg-app-accent/70 opacity-90" />
              <span className="flow-line-shot-delay absolute left-0 top-0 h-10 w-full bg-app-accent/40 opacity-85" />
            </div>
            <div className="relative space-y-8">
              {steps.map((step, index) => (
                <div key={step.title} className="relative flex items-start gap-4">
                  <div
                    className={`relative z-10 flex h-[54px] w-[54px] items-center justify-center rounded-full border ${
                      step.state === 'done' ? 'flow-node-pulse' : ''
                    }`}
                    style={{
                      backgroundColor: 'rgb(var(--color-app-panel-soft))',
                      borderColor: step.color,
                      opacity: step.state === 'done' ? 1 : 0.55,
                    }}
                  >
                    <div
                      className="flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold text-white"
                      style={{ backgroundColor: step.color }}
                    >
                      {index + 1}
                    </div>
                  </div>
                  <div className="pb-2 pt-1">
                    <div className="flex items-center gap-2">
                      <p className="text-sm font-semibold text-app-text">{step.title}</p>
                      <span className="rounded-md border border-app-border bg-app-panel px-1.5 py-0.5 text-[11px] text-app-muted">
                        {step.state === 'done' ? 'done' : 'waiting'}
                      </span>
                    </div>
                    <p className="mt-1 font-mono text-xs text-app-muted">{step.detail}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}

function FlowMetric({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="rounded-xl border border-app-border bg-app-panel p-3">
      <div className="text-[10px] font-black uppercase tracking-widest text-app-muted">{label}</div>
      <div className={`mt-1 text-[20px] font-black ${color}`}>{value}</div>
    </div>
  );
}
