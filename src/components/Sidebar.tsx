import { BookOpen, ChevronDown, ChevronRight, FileCode2, Folder, Search } from 'lucide-react';
import { useState } from 'react';
import { useAppStore } from '../store/useAppStore';

const explorerItems = [
  {
    group: 'frontend',
    files: [
      { path: 'src/App.tsx', label: 'App.tsx', color: 'text-blue-400' },
      {
        path: 'src/components/MainPanel.tsx',
        label: 'components/MainPanel.tsx',
        color: 'text-violet-400',
      },
      {
        path: 'src/components/RightPanel.tsx',
        label: 'components/RightPanel.tsx',
        color: 'text-violet-400',
      },
      {
        path: 'src/components/GraphView.tsx',
        label: 'components/GraphView.tsx',
        color: 'text-blue-400',
      },
    ],
  },
  {
    group: 'backend',
    files: [
      {
        path: 'src-tauri/src/commands/mod.rs',
        label: 'commands/mod.rs',
        color: 'text-emerald-400',
      },
      { path: 'src-tauri/src/graph/store.rs', label: 'graph/store.rs', color: 'text-amber-400' },
      { path: 'src-tauri/src/llm/mod.rs', label: 'llm/mod.rs', color: 'text-red-400' },
    ],
  },
];

const wikiItems = [
  'project-overview.md',
  'architecture.md',
  'features/03-tree-sitter-parser.md',
  'features/10-hybrid-search.md',
  'features/16-graph-visualization.md',
];

export function Sidebar() {
  const {
    activeSourcePath,
    indexPathSummary,
    searchIndexSummary,
    setActivePanel,
    setActiveSourcePath,
  } = useAppStore();
  const [tab, setTab] = useState<'explorer' | 'wiki'>('explorer');
  const indexedCount = searchIndexSummary?.documentsIndexed ?? indexPathSummary?.filesSeen ?? 0;

  return (
    <aside className="flex h-full min-w-0 flex-col border-r border-app-border bg-app-panel">
      <div className="border-b border-app-border p-3">
        <div className="flex items-center gap-2">
          <div className="h-5 w-5 rounded-sm bg-gradient-to-br from-blue-500 to-violet-500" />
          <h1 className="text-[15px] font-semibold tracking-tight">localbrain</h1>
        </div>
        <div className="mt-3 flex h-8 items-center justify-between rounded-lg border border-app-border bg-app-background px-2.5 text-xs">
          <span className="flex items-center gap-1.5">
            <span className="h-2 w-2 rounded-full bg-emerald-500" />
            localbrain · main
          </span>
          <ChevronDown className="h-3.5 w-3.5 text-app-muted" aria-hidden="true" />
        </div>
        <div className="mt-2 inline-flex items-center gap-1.5 rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2 py-1 text-[11px] text-emerald-400">
          <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
          Indexed {indexedCount || 0} files · 100% local
        </div>
      </div>

      <div className="flex items-center gap-1 border-b border-app-border px-2 pt-2 text-xs font-medium">
        <button
          className="border-b-2 border-transparent px-3 py-2 text-app-muted hover:text-app-text data-[active=true]:border-app-accent data-[active=true]:text-app-text"
          data-active={tab === 'explorer'}
          type="button"
          onClick={() => setTab('explorer')}
        >
          Explorer
        </button>
        <button
          className="border-b-2 border-transparent px-3 py-2 text-app-muted hover:text-app-text data-[active=true]:border-app-accent data-[active=true]:text-app-text"
          data-active={tab === 'wiki'}
          type="button"
          onClick={() => {
            setTab('wiki');
            setActivePanel('wiki');
          }}
        >
          Wiki
        </button>
      </div>

      <div className="app-scrollbar min-h-0 flex-1 overflow-y-auto p-2 text-xs">
        {tab === 'explorer' ? (
          <div>
            <div className="px-1.5 py-1 text-[11px] uppercase tracking-wider text-app-muted">
              SRC
            </div>
            <div className="space-y-1">
              {explorerItems.map((group) => (
                <div key={group.group}>
                  <div className="flex items-center gap-1.5 rounded px-2 py-1 text-app-muted">
                    <ChevronRight className="h-3.5 w-3.5" aria-hidden="true" />
                    <Folder className="h-3.5 w-3.5" aria-hidden="true" />
                    {group.group}
                  </div>
                  <div className="ml-4 space-y-0.5">
                    {group.files.map((file) => (
                      <button
                        key={file.path}
                        className="flex min-h-7 w-full items-center gap-1.5 rounded px-2 text-left text-app-muted hover:bg-app-panelSoft hover:text-app-text data-[active=true]:bg-app-panelSoft data-[active=true]:text-app-text"
                        data-active={file.path === activeSourcePath}
                        type="button"
                        onClick={() => {
                          setActiveSourcePath(file.path);
                          setActivePanel('graph');
                        }}
                      >
                        <FileCode2
                          className={`h-3.5 w-3.5 shrink-0 ${file.color}`}
                          aria-hidden="true"
                        />
                        <span className="truncate">{file.label}</span>
                      </button>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div>
            <div className="px-1.5 py-1 text-[11px] uppercase tracking-wider text-app-muted">
              docs/wiki
            </div>
            <div className="space-y-1">
              {wikiItems.map((item) => (
                <button
                  key={item}
                  className="flex min-h-8 w-full items-center gap-2 rounded px-2 text-left text-app-muted hover:bg-app-panelSoft hover:text-app-text"
                  type="button"
                  onClick={() => setActivePanel('wiki')}
                >
                  <BookOpen className="h-3.5 w-3.5 shrink-0 text-app-muted" aria-hidden="true" />
                  <span className="truncate">{item}</span>
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="border-t border-app-border p-2">
        <div className="px-1.5 py-1 text-[11px] uppercase tracking-wider text-app-muted">
          .localbrain
        </div>
        <div className="grid grid-cols-2 gap-1.5 p-1.5">
          <Metric label="graph/" value={`${indexedCount || 0} nodes`} color="bg-blue-500" />
          <Metric label="wiki/" value="pages" color="bg-violet-500" />
          <Metric label="metadata/" value="local" />
          <Metric label="search/" value="private" />
        </div>
        <div className="mt-2 flex h-9 items-center gap-2 rounded-md border border-app-border bg-app-background px-3 text-app-muted">
          <Search className="h-4 w-4" aria-hidden="true" />
          <span className="text-xs">Command palette</span>
          <kbd className="ml-auto rounded border border-app-border px-1.5 py-0.5 font-mono text-[10px]">
            ⌘K
          </kbd>
        </div>
      </div>
    </aside>
  );
}

function Metric({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div className="rounded-md border border-app-border bg-app-background p-2">
      <div className="text-[10px] text-app-muted">{label}</div>
      <div className="mt-1 text-[11px] text-app-text">{value}</div>
      {color ? (
        <div className="mt-1.5 h-1 overflow-hidden rounded bg-app-panelSoft">
          <div className={`h-full w-4/5 ${color}`} />
        </div>
      ) : null}
    </div>
  );
}
