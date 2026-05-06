import { useEffect, useState, useCallback } from 'react';
import { Download, FileText, GitBranch, Loader2, Search, Workflow, X } from 'lucide-react';
import { FlowView } from './FlowView';
import { GraphView } from './GraphView';
import { WikiView } from './WikiView';
import type { GraphViewData } from '../lib/graph';
import { getGraphView } from '../lib/graph';
import { generate_wiki } from '../lib/wiki';
import { useAppStore } from '../store/useAppStore';

const views = [
  { id: 'graph', label: 'Graph View' },
  { id: 'wiki', label: 'Wiki View' },
  { id: 'flow', label: 'Flow View' },
] as const;

export function MainPanel() {
  const [isCommandOpen, setIsCommandOpen] = useState(false);
  const {
    activePanel,
    activeSourcePath,
    projectPath,
    isProjectLoading,
    graphView,
    setActivePanel,
    setGraphError,
    setGraphView,
    setSelectedGraphNode,
    setWikiError,
    setWikiResult,
  } = useAppStore();

  const handleOpenGraph = useCallback(async () => {
    try {
      const view = await getGraphView(activeSourcePath, 40);
      setGraphView(view);
      setActivePanel('graph');
    } catch (error) {
      setGraphError(error instanceof Error ? error.message : String(error));
    }
  }, [activeSourcePath, setActivePanel, setGraphError, setGraphView]);

  useEffect(() => {
    if (activePanel === 'graph') {
      void handleOpenGraph();
    }
  }, [activePanel, activeSourcePath, handleOpenGraph]);

  async function handleExportWiki() {
    if (!projectPath || isProjectLoading) {
      return;
    }
    try {
      const summary = await generate_wiki(projectPath);
      setWikiResult(summary);
      setActivePanel('wiki');
    } catch (error) {
      setWikiError(error instanceof Error ? error.message : String(error));
    }
  }

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        setIsCommandOpen((open) => !open);
      }
      if (event.key === 'Escape') {
        setIsCommandOpen(false);
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const commandItems = [
    {
      label: 'Open Graph View',
      detail: 'Load knowledge graph for the active source',
      icon: GitBranch,
      run: () => void handleOpenGraph(),
    },
    {
      label: 'Open Wiki View',
      detail: 'Read generated source-backed notes',
      icon: FileText,
      run: () => setActivePanel('wiki'),
    },
    {
      label: 'Open Flow View',
      detail: 'Show local parsing and retrieval flow',
      icon: Workflow,
      run: () => setActivePanel('flow'),
    },
    {
      label: 'Export Wiki',
      detail: 'Generate wiki pages from the current workspace',
      icon: Download,
      run: () => void handleExportWiki(),
    },
  ].filter((item) => (item.label === 'Export Wiki' ? Boolean(projectPath) : true));

  const dynamicLegendItems = buildDynamicLegend(graphView);

  return (
    <main className="flex min-w-0 flex-col bg-app-background">
      <div className="flex h-14 items-center justify-between border-b border-app-border bg-app-panel/80 px-4 backdrop-blur-md">
        <div className="flex items-center gap-8">
          <div className="flex items-center gap-2">
            {views.map((view) => (
              <button
                key={view.id}
                className="border-b-2 border-transparent px-4 py-3 text-[14px] font-bold text-app-muted hover:text-app-text transition-all data-[active=true]:border-app-accent data-[active=true]:text-app-text"
                data-active={activePanel === view.id}
                type="button"
                onClick={() => {
                  setActivePanel(view.id);
                  if (view.id === 'graph') {
                    void handleOpenGraph();
                  }
                }}
              >
                {view.label}
              </button>
            ))}
          </div>
          <div className="hidden items-center gap-5 text-[12px] font-bold text-app-muted lg:flex">
            {dynamicLegendItems.map((item) => (
              <Legend key={item.kind} label={item.label} colorVar={item.colorVar} />
            ))}
          </div>
        </div>
        <div className="flex items-center gap-3">
          <button
            className="flex h-9 items-center gap-2 rounded-lg border border-app-border bg-app-panel px-3.5 text-[13px] font-bold hover:bg-app-panelSoft transition-colors"
            type="button"
            onClick={handleExportWiki}
            disabled={!projectPath || isProjectLoading}
          >
            {isProjectLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
            ) : (
              <Download className="h-4 w-4" aria-hidden="true" />
            )}
            {isProjectLoading ? 'Loading Project' : 'Export Wiki'}
          </button>
          <button
            className="flex h-9 items-center gap-2 rounded-lg border border-app-border bg-app-panel px-3.5 text-[13px] font-bold hover:bg-app-panelSoft transition-colors"
            type="button"
            onClick={() => setIsCommandOpen(true)}
          >
            <Search className="h-4 w-4" aria-hidden="true" />
            ⌘K
          </button>
        </div>
      </div>

      <section className="relative min-h-0 flex-1 overflow-hidden">
        {activePanel === 'graph' ? (
          <GraphView data={graphView} onSelectNode={setSelectedGraphNode} />
        ) : null}
        {activePanel === 'wiki' ? <WikiView /> : null}
        {activePanel === 'flow' ? <FlowView /> : null}
      </section>

      <div className="flex h-8 items-center justify-between border-t border-app-border bg-app-panel px-4 text-[11px] font-bold text-app-muted">
        <span>ALL DATA RUNS LOCALLY · SQLITE + GRAPH STORE · NO CLOUD SYNC</span>
        <span className="bg-app-panelSoft px-2 py-0.5 rounded border border-app-border">
          v0.1.0
        </span>
      </div>

      {isCommandOpen ? (
        <div className="fixed inset-0 z-50 flex items-start justify-center bg-black/60 backdrop-blur-sm pt-[15vh]">
          <div className="w-[600px] overflow-hidden rounded-2xl border border-app-border bg-app-panel shadow-[0_0_50px_-12px_rgba(0,0,0,0.5)]">
            <div className="flex h-14 items-center gap-3 border-b border-app-border px-4">
              <Search className="h-5 w-5 text-app-muted" aria-hidden="true" />
              <span className="text-base font-bold">Command palette</span>
              <button
                className="ml-auto rounded-lg p-1.5 text-app-muted hover:bg-app-panelSoft hover:text-app-text transition-colors"
                type="button"
                aria-label="Close command palette"
                onClick={() => setIsCommandOpen(false)}
              >
                <X className="h-5 w-5" aria-hidden="true" />
              </button>
            </div>
            <div className="p-3">
              {commandItems.map((item) => {
                const Icon = item.icon;

                return (
                  <button
                    key={item.label}
                    className="flex w-full items-center gap-4 rounded-xl px-4 py-3 text-left hover:bg-app-panelSoft group transition-all"
                    type="button"
                    onClick={() => {
                      item.run();
                      setIsCommandOpen(false);
                    }}
                  >
                    <div className="p-2.5 rounded-lg bg-app-panel group-hover:bg-app-background transition-colors">
                      <Icon className="h-5 w-5 text-app-accent" aria-hidden="true" />
                    </div>
                    <span className="min-w-0">
                      <span className="block text-[15px] font-bold text-app-text">
                        {item.label}
                      </span>
                      <span className="block truncate text-[13px] font-medium text-app-muted mt-0.5">
                        {item.detail}
                      </span>
                    </span>
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      ) : null}
    </main>
  );
}

function Legend({ label, colorVar }: { label: string; colorVar: string }) {
  return (
    <span className="flex items-center gap-2">
      <span
        className="h-2 w-2 rounded-full shadow-[0_0_8px_currentColor]"
        style={{ backgroundColor: `rgb(var(${colorVar}))`, color: `rgb(var(${colorVar}))` }}
      />
      {label}
    </span>
  );
}

const KIND_META: Record<string, { label: string; colorVar: string }> = {
  file: { label: 'File', colorVar: '--color-app-text' },
  component: { label: 'Component', colorVar: '--color-graph-component' },
  import: { label: 'Import / API', colorVar: '--color-graph-api' },
  hook: { label: 'Hook', colorVar: '--color-graph-hook' },
  external_library: { label: 'External Lib', colorVar: '--color-graph-external' },
  class: { label: 'Type / Model', colorVar: '--color-graph-model' },
  interface: { label: 'Type / Model', colorVar: '--color-graph-model' },
  type_alias: { label: 'Type / Model', colorVar: '--color-graph-model' },
  enum: { label: 'Type / Model', colorVar: '--color-graph-model' },
  method: { label: 'Function / Service', colorVar: '--color-graph-service' },
  function: { label: 'Function / Service', colorVar: '--color-graph-service' },
  object: { label: 'Feature / Object', colorVar: '--color-graph-feature' },
  export: { label: 'Export', colorVar: '--color-graph-api' },
};

function buildDynamicLegend(graphView: GraphViewData | null) {
  const fallbackKinds = ['component', 'import', 'hook', 'external_library', 'class', 'function'];

  const kinds = new Set<string>(
    graphView?.nodes
      .map((node: GraphViewData['nodes'][number]) => node.kind)
      .filter((kind: string) => kind !== 'file') ?? fallbackKinds,
  );

  const orderedKinds = [
    'component',
    'import',
    'hook',
    'external_library',
    'class',
    'interface',
    'type_alias',
    'enum',
    'function',
    'method',
    'object',
    'export',
  ].filter((kind) => kinds.has(kind));

  const dedupedByLabel = new Map<string, { kind: string; label: string; colorVar: string }>();
  for (const kind of orderedKinds) {
    const meta = KIND_META[kind];
    if (!meta) continue;
    const key = `${meta.label}-${meta.colorVar}`;
    if (!dedupedByLabel.has(key)) {
      dedupedByLabel.set(key, { kind, ...meta });
    }
  }

  return Array.from(dedupedByLabel.values());
}
