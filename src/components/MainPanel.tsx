import { useEffect, useState } from 'react';
import { Download, FileText, GitBranch, Search, Workflow, X } from 'lucide-react';
import { FlowView } from './FlowView';
import { GraphView } from './GraphView';
import { WikiView } from './WikiView';
import { getGraphView } from '../lib/graph';
import { generateWiki } from '../lib/wiki';
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
    graphView,
    setActivePanel,
    setGraphError,
    setGraphView,
    setSelectedGraphNode,
    setWikiError,
    setWikiResult,
  } = useAppStore();

  async function handleOpenGraph() {
    try {
      const view = await getGraphView(activeSourcePath, 40);
      setGraphView(view);
      setActivePanel('graph');
    } catch (error) {
      setGraphError(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleExportWiki() {
    try {
      const summary = await generateWiki('.');
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
  ];

  return (
    <main className="flex min-w-0 flex-col bg-app-background">
      <div className="flex h-11 items-center justify-between border-b border-app-border bg-app-panel/70 px-3">
        <div className="flex items-center gap-5">
          <div className="flex items-center gap-1">
            {views.map((view) => (
              <button
                key={view.id}
                className="border-b-2 border-transparent px-3 py-2 text-[13px] font-medium text-app-muted hover:text-app-text data-[active=true]:border-app-accent data-[active=true]:text-app-text"
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
          <div className="hidden items-center gap-2 text-[11px] text-app-muted lg:flex">
            <Legend label="Feature" color="bg-blue-500" />
            <Legend label="Component" color="bg-violet-500" />
            <Legend label="API" color="bg-emerald-500" />
            <Legend label="Service" color="bg-amber-500" />
            <Legend label="Model" color="bg-red-500" />
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            className="flex h-8 items-center gap-1.5 rounded-md border border-app-border bg-app-panel px-2.5 text-xs hover:bg-app-panelSoft"
            type="button"
            onClick={handleExportWiki}
          >
            <Download className="h-3.5 w-3.5" aria-hidden="true" />
            Export Wiki
          </button>
          <button
            className="flex h-8 items-center gap-1.5 rounded-md border border-app-border bg-app-panel px-2.5 text-xs hover:bg-app-panelSoft"
            type="button"
            onClick={() => setIsCommandOpen(true)}
          >
            <Search className="h-3.5 w-3.5" aria-hidden="true" />
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

      <div className="flex h-6 items-center justify-between border-t border-app-border bg-app-panel px-3 text-[10px] text-app-muted">
        <span>All data runs locally · SQLite + graph store · no cloud sync</span>
        <span>v0.1.0</span>
      </div>

      {isCommandOpen ? (
        <div className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-[12vh]">
          <div className="w-[520px] overflow-hidden rounded-xl border border-app-border bg-app-panel shadow-2xl">
            <div className="flex h-11 items-center gap-2 border-b border-app-border px-3">
              <Search className="h-4 w-4 text-app-muted" aria-hidden="true" />
              <span className="text-sm font-medium">Command palette</span>
              <button
                className="ml-auto rounded-md p-1 text-app-muted hover:bg-app-panelSoft hover:text-app-text"
                type="button"
                aria-label="Close command palette"
                onClick={() => setIsCommandOpen(false)}
              >
                <X className="h-4 w-4" aria-hidden="true" />
              </button>
            </div>
            <div className="p-2">
              {commandItems.map((item) => {
                const Icon = item.icon;

                return (
                  <button
                    key={item.label}
                    className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left hover:bg-app-panelSoft"
                    type="button"
                    onClick={() => {
                      item.run();
                      setIsCommandOpen(false);
                    }}
                  >
                    <Icon className="h-4 w-4 text-app-accent" aria-hidden="true" />
                    <span className="min-w-0">
                      <span className="block text-sm font-medium text-app-text">{item.label}</span>
                      <span className="block truncate text-xs text-app-muted">{item.detail}</span>
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

function Legend({ label, color }: { label: string; color: string }) {
  return (
    <span className="flex items-center gap-1">
      <span className={`h-1.5 w-1.5 rounded-full ${color}`} />
      {label}
    </span>
  );
}
