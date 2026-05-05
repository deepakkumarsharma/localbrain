import { BookOpen, ChevronDown, ChevronRight, FileCode2, Folder, Search } from 'lucide-react';
import { useMemo, useState } from 'react';
import logo from '../assets/logo.png';
import { useAppStore } from '../store/useAppStore';

interface FileTreeGroup {
  name: string;
  files: { path: string; label: string; color: string }[];
}

export function Sidebar() {
  const {
    activeSourcePath,
    indexPathSummary,
    searchIndexSummary,
    setActivePanel,
    setActiveSourcePath,
  } = useAppStore();
  const [tab, setTab] = useState<'explorer' | 'wiki'>('explorer');
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set(['root', 'src']));
  const indexedCount = searchIndexSummary?.documentsIndexed ?? indexPathSummary?.filesSeen ?? 0;

  const toggleGroup = (name: string) => {
    const next = new Set(expandedGroups);
    if (next.has(name)) {
      next.delete(name);
    } else {
      next.add(name);
    }
    setExpandedGroups(next);
  };

  const dynamicGroups = useMemo(() => {
    if (!indexPathSummary?.files) {
      return [];
    }

    const groups: Record<string, FileTreeGroup> = {};

    indexPathSummary.files.forEach((file) => {
      const parts = file.path.split('/');
      let groupName = 'root';
      let label = file.path;

      if (parts.length > 1) {
        groupName = parts[0];
        label = parts.slice(1).join('/');
      }

      if (!groups[groupName]) {
        groups[groupName] = { name: groupName, files: [] };
      }

      groups[groupName].files.push({
        path: file.path,
        label: label,
        color: getFileColor(file.path),
      });
    });

    return Object.values(groups).sort((a, b) => {
      if (a.name === 'root') return -1;
      if (b.name === 'root') return 1;
      return a.name.localeCompare(b.name);
    });
  }, [indexPathSummary]);

  const wikiItems = useMemo(() => {
    if (!indexPathSummary?.files) return [];
    return indexPathSummary.files
      .filter((f) => f.path.endsWith('.ts') || f.path.endsWith('.tsx') || f.path.endsWith('.js'))
      .map((f) => ({
        path: f.path,
        label: `${f.path.split('/').join('_')}.md`,
      }));
  }, [indexPathSummary]);

  return (
    <aside className="flex h-full min-w-0 flex-col border-r border-app-border bg-app-panel overflow-hidden">
      {/* Top Header Section - Fixed */}
      <div className="shrink-0 border-b border-app-border p-4 z-20 bg-app-panel">
        <div className="flex items-center gap-2.5">
          <img src={logo} alt="Local Brain Logo" className="h-6 w-6 rounded-md object-contain" />
          <h1 className="text-[18px] font-bold tracking-tight text-app-text uppercase">
            Local Brain
          </h1>
        </div>
        <div className="mt-4 flex h-9 items-center justify-between rounded-lg border border-app-border bg-app-background px-3 text-[12px] font-bold">
          <span className="flex min-w-0 items-center gap-2 overflow-hidden">
            <span className="h-2.5 w-2.5 shrink-0 rounded-full bg-emerald-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]" />
            <span className="truncate" title="localbrain · main">
              localbrain · main
            </span>
          </span>
          <ChevronDown className="h-4 w-4 shrink-0 text-app-muted" aria-hidden="true" />
        </div>
        <div className="mt-2.5 inline-flex items-center gap-2 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-3 py-1.5 text-[12px] font-bold text-emerald-400">
          <span className="h-2 w-2 rounded-full bg-emerald-500" />
          Indexed {indexedCount || 0} files · 100% local
        </div>
      </div>

      {/* Tabs Section - Fixed */}
      <div className="shrink-0 flex items-center gap-1 border-b border-app-border px-3 pt-3 text-[13px] font-bold z-20 bg-app-panel">
        <button
          className="border-b-2 border-transparent px-4 py-2.5 text-app-muted hover:text-app-text data-[active=true]:border-app-accent data-[active=true]:text-app-text transition-all"
          data-active={tab === 'explorer'}
          type="button"
          onClick={() => setTab('explorer')}
        >
          Explorer
        </button>
        <button
          className="border-b-2 border-transparent px-4 py-2.5 text-app-muted hover:text-app-text data-[active=true]:border-app-accent data-[active=true]:text-app-text transition-all"
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

      {/* Scrollable Container */}
      <div className="flex-1 overflow-y-auto relative app-scrollbar flex flex-col">
        {/* Main Content */}
        <div className="p-3 text-[14px] flex-1">
          {tab === 'explorer' ? (
            <div className="pb-4">
              <div className="px-2 py-1.5 text-[12px] font-black uppercase tracking-widest text-app-muted/70">
                WORKSPACE
              </div>
              <div className="space-y-1">
                {dynamicGroups.map((group) => {
                  const isExpanded = expandedGroups.has(group.name);
                  return (
                    <div key={group.name}>
                      <button
                        className="flex w-full items-center gap-2 rounded px-2 py-1.5 font-bold text-app-text/90 hover:bg-app-panelSoft transition-colors"
                        type="button"
                        onClick={() => toggleGroup(group.name)}
                      >
                        <ChevronRight
                          className={`h-4 w-4 transition-transform ${isExpanded ? 'rotate-90' : ''}`}
                          aria-hidden="true"
                        />
                        <Folder className="h-4 w-4 text-app-accent" aria-hidden="true" />
                        {group.name}
                      </button>
                      {isExpanded && (
                        <div className="ml-5 space-y-0.5">
                          {group.files.map((file) => (
                            <button
                              key={file.path}
                              className="flex min-h-8 w-full items-center gap-2 rounded px-2 text-left font-medium text-app-muted hover:bg-app-panelSoft hover:text-app-text data-[active=true]:bg-app-panelSoft data-[active=true]:text-app-text data-[active=true]:font-bold transition-all"
                              data-active={file.path === activeSourcePath}
                              type="button"
                              onClick={() => {
                                setActiveSourcePath(file.path);
                                setActivePanel('graph');
                              }}
                            >
                              <FileCode2
                                className={`h-4 w-4 shrink-0 ${file.color}`}
                                aria-hidden="true"
                              />
                              <span className="truncate" title={file.label}>
                                {file.label}
                              </span>
                            </button>
                          ))}
                        </div>
                      )}
                    </div>
                  );
                })}
                {dynamicGroups.length === 0 && (
                  <div className="p-6 text-center text-app-muted font-medium">
                    Indexing workspace...
                  </div>
                )}
              </div>
            </div>
          ) : (
            <div className="pb-4">
              <div className="px-2 py-1.5 text-[12px] font-black uppercase tracking-widest text-app-muted/70">
                DOCS/WIKI
              </div>
              <div className="space-y-1">
                {wikiItems.map((item) => (
                  <button
                    key={item.path}
                    className="flex min-h-9 w-full items-center gap-2.5 rounded px-2 text-left font-medium text-app-muted hover:bg-app-panelSoft hover:text-app-text transition-all"
                    type="button"
                    onClick={() => {
                      setActiveSourcePath(item.path);
                      setActivePanel('wiki');
                    }}
                  >
                    <BookOpen className="h-4 w-4 shrink-0 text-app-muted" aria-hidden="true" />
                    <span className="truncate" title={item.label}>
                      {item.label}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Fixed Metrics Section - Sticky at bottom of scrollable area */}
        <div className="sticky bottom-0 shrink-0 border-t border-app-border p-4 bg-app-panel/95 backdrop-blur-md shadow-[0_-10px_30px_-15px_rgba(0,0,0,0.7)] z-10">
          <div className="px-2 py-1.5 text-[11px] font-black uppercase tracking-widest text-app-muted/70">
            .LOCALBRAIN
          </div>
          <div className="grid grid-cols-2 gap-2 p-1">
            <Metric label="GRAPH" value={`${indexedCount || 0} nodes`} color="bg-blue-500" />
            <Metric label="WIKI" value={`${wikiItems.length} pages`} color="bg-violet-500" />
          </div>
          <div className="mt-3 flex h-11 items-center gap-2.5 rounded-xl border border-app-border bg-app-background px-3.5 text-app-muted hover:text-app-text hover:border-app-accent/50 transition-all cursor-pointer group shadow-inner">
            <Search
              className="h-5 w-5 group-hover:text-app-accent transition-colors"
              aria-hidden="true"
            />
            <span className="text-[14px] font-bold">Command palette</span>
            <kbd className="ml-auto rounded-lg border border-app-border bg-app-panelSoft px-2 py-1 font-mono text-[12px] font-bold shadow-sm">
              ⌘K
            </kbd>
          </div>
        </div>
      </div>
    </aside>
  );
}

function getFileColor(path: string) {
  if (path.endsWith('.tsx') || path.endsWith('.ts')) return 'text-blue-400';
  if (path.endsWith('.rs')) return 'text-orange-400';
  if (path.endsWith('.json')) return 'text-yellow-400';
  if (path.endsWith('.md')) return 'text-emerald-400';
  return 'text-app-muted';
}

function Metric({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div className="rounded-xl border border-app-border bg-app-background p-3 shadow-sm">
      <div className="text-[10px] font-black tracking-widest text-app-muted uppercase">{label}</div>
      <div className="mt-1 truncate text-[14px] font-bold text-app-text" title={value}>
        {value}
      </div>
      {color ? (
        <div className="mt-2.5 h-1.5 overflow-hidden rounded-full bg-app-panelSoft">
          <div
            className={`h-full w-4/5 ${color} shadow-[0_0_10px_rgba(var(--color-app-accent),0.4)]`}
          />
        </div>
      ) : null}
    </div>
  );
}
