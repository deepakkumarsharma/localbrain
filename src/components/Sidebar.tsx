import {
  BookOpen,
  Brain,
  Search,
  FolderOpen,
  ChevronRight,
  FileCode2,
  Folder,
  Loader2,
  Play,
  Trash2,
  Square,
} from 'lucide-react';
import { useMemo, useState, useEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import logo from '../assets/logo.png';
import {
  getLocalLlmStatus,
  getProviderSettings,
  setLocalModelPath,
  startLocalLlm,
  stopLocalLlm,
} from '../lib/settings';
import { useAppStore } from '../store/useAppStore';

interface FileTreeNode {
  name: string;
  path: string;
  kind: 'folder' | 'file';
  color?: string;
  children: FileTreeNode[];
}

interface SidebarProps {
  onSelectProject: (path: string) => Promise<void>;
  onRemoveProject: () => void;
}

const WIKI_SOURCE_FILE_PATTERN =
  /\.(js|mjs|cjs|jsx|ts|mts|cts|tsx|rs|go|py|java|kt|kts|swift|rb|php|c|h|cpp|hpp|cs|sh|bash|zsh|fish|sql|json|jsonc|ya?ml|toml|ini|cfg|conf|xml|css|scss|less|vue|svelte|astro)$/i;

export function Sidebar({ onSelectProject, onRemoveProject }: SidebarProps) {
  const {
    activeSourcePath,
    indexPathSummary,
    searchIndexSummary,
    providerSettings,
    llmRunning,
    theme,
    toggleTheme,
    projectPath,
    isProjectLoading,
    projectStatus,
    citations,
    setActivePanel,
    setActiveSourcePath,
    setProviderSettings,
    setLlmRunning,
  } = useAppStore();
  const [tab, setTab] = useState<'explorer' | 'wiki' | 'sources'>('explorer');
  const [isPickingProject, setIsPickingProject] = useState(false);
  const [explorerQuery, setExplorerQuery] = useState('');
  const [isStartingServer, setIsStartingServer] = useState(false);
  const [serverStatus, setServerStatus] = useState<string | null>(null);
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set(['']));
  const indexedCount = searchIndexSummary?.documentsIndexed ?? indexPathSummary?.filesSeen ?? 0;

  useEffect(() => {
    void getProviderSettings().then(setProviderSettings).catch(console.error);
    void getLocalLlmStatus().then(setLlmRunning).catch(console.error);
  }, [setLlmRunning, setProviderSettings]);

  const pickProjectFolder = async () => {
    setIsPickingProject(true);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (selected && typeof selected === 'string') {
        await onSelectProject(selected);
      }
    } catch (error) {
      console.error('Failed to select project folder:', error);
    } finally {
      setIsPickingProject(false);
    }
  };

  const selectModel = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Model', extensions: ['gguf'] }],
      });
      if (selected && typeof selected === 'string') {
        const settings = await setLocalModelPath(selected);
        setProviderSettings(settings);
      }
    } catch (error) {
      console.error('Failed to select model:', error);
    }
  };

  const startServer = async () => {
    setIsStartingServer(true);
    setServerStatus('Starting local server...');
    try {
      await startLocalLlm();
      let running = false;
      for (let attempt = 0; attempt < 40; attempt += 1) {
        running = await getLocalLlmStatus();
        if (running) {
          break;
        }
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
      setLlmRunning(running);
      if (!running) {
        setServerStatus('Server started but not healthy yet. Please wait...');
      } else {
        setServerStatus('Local server is ready.');
      }
    } catch (error) {
      setServerStatus(`Failed to start server: ${error}`);
    } finally {
      setIsStartingServer(false);
    }
  };

  const stopServer = async () => {
    try {
      await stopLocalLlm();
      setLlmRunning(false);
      setServerStatus('Local server stopped.');
    } catch (error) {
      setServerStatus(`Failed to stop server: ${error}`);
    }
  };

  const toggleGroup = (path: string) => {
    const next = new Set(expandedGroups);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    setExpandedGroups(next);
  };

  const fileTree = useMemo(() => {
    const paths = indexPathSummary?.files?.map((file) => file.path) ?? [];
    const query = explorerQuery.trim().toLowerCase();
    const filtered =
      query.length > 0 ? paths.filter((path) => path.toLowerCase().includes(query)) : paths;
    return buildFileTree(filtered);
  }, [indexPathSummary, explorerQuery]);

  const wikiItems = useMemo(() => {
    if (!indexPathSummary?.files) return [];
    return indexPathSummary.files
      .filter((f) => WIKI_SOURCE_FILE_PATTERN.test(f.path))
      .map((f) => ({
        path: f.path,
        label: `${f.path.split('/').join('_')}.md`,
      }));
  }, [indexPathSummary]);

  const sourceItems = useMemo(() => {
    if (!projectPath) return [];
    const mapped =
      citations.length > 0
        ? citations.map((item) => ({
            path: item.path,
            label: sourceLabel(item.path, item.startLine, item.endLine),
            title: item.title || item.path.split('/').pop() || item.path,
          }))
        : [];
    const seen = new Set<string>();
    return mapped.filter((item) => {
      if (seen.has(item.label)) return false;
      seen.add(item.label);
      return true;
    });
  }, [citations, projectPath]);

  useEffect(() => {
    if (fileTree.length === 0) return;
    setExpandedGroups((existing) => {
      const next = new Set(existing);
      for (const node of fileTree) {
        if (node.kind === 'folder') {
          next.add(node.path);
        }
      }
      return next;
    });
  }, [fileTree]);

  return (
    <aside className="flex h-full min-w-0 flex-col border-r border-app-border bg-app-panel overflow-hidden">
      {/* Top Header Section - Fixed */}
      <div className="shrink-0 border-b border-app-border p-4 z-20 bg-app-panel">
        <div className="flex items-center gap-3">
          <div className="rounded-xl bg-gradient-to-br from-blue-500/30 to-violet-500/30 p-1.5 ring-1 ring-app-border">
            <img src={logo} alt="Local Brain Logo" className="h-6 w-6 rounded-md object-contain" />
          </div>
          <h1 className="text-[18px] font-black tracking-tight text-app-text uppercase">
            Local Brain
          </h1>
        </div>
        <div className="mt-4 space-y-2">
          {!projectPath ? (
            <button
              className="flex h-10 w-full items-center justify-center gap-2 rounded-lg border border-app-border bg-app-panelSoft px-3 text-[12px] font-bold text-app-text hover:border-app-accent disabled:opacity-60"
              type="button"
              disabled={isPickingProject || isProjectLoading}
              onClick={() => void pickProjectFolder()}
            >
              {isPickingProject || isProjectLoading ? (
                <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
              ) : (
                <FolderOpen className="h-4 w-4" aria-hidden="true" />
              )}
              Select Project
            </button>
          ) : (
            <div className="grid grid-cols-2 gap-2">
              <button
                className="flex h-9 w-full items-center justify-center gap-2 rounded-lg border border-app-border bg-app-panelSoft px-3 text-[12px] font-bold text-app-text hover:border-app-accent disabled:opacity-60"
                type="button"
                disabled={isPickingProject || isProjectLoading}
                onClick={() => void pickProjectFolder()}
              >
                <FolderOpen className="h-4 w-4" aria-hidden="true" />
                Change
              </button>
              <button
                className="flex h-9 w-full items-center justify-center gap-2 rounded-lg border border-app-border bg-app-background px-3 text-[12px] font-bold text-app-muted hover:text-app-text disabled:opacity-50"
                type="button"
                disabled={!projectPath || isProjectLoading}
                onClick={onRemoveProject}
              >
                <Trash2 className="h-4 w-4" aria-hidden="true" />
                Remove
              </button>
            </div>
          )}
          {projectPath ? (
            <div className="rounded-lg border border-app-border bg-app-background p-2.5">
              <div className="text-[10px] font-black uppercase tracking-widest text-app-muted">
                Active Workspace
              </div>
              <div
                className="mt-1 truncate font-mono text-[12px] text-app-text"
                title={projectPath}
              >
                {projectPath}
              </div>
            </div>
          ) : null}
          {projectStatus ? (
            <div className="flex flex-wrap gap-2 text-[10px] font-black uppercase tracking-widest">
              <span
                className={`rounded-full border px-2 py-1 ${isProjectLoading ? 'border-blue-500/30 bg-blue-500/10 text-blue-400' : 'border-emerald-500/30 bg-emerald-500/10 text-emerald-400'}`}
              >
                {isProjectLoading ? 'Indexing' : 'Ready'}
              </span>
              <span className="rounded-full border border-violet-500/30 bg-violet-500/10 px-2 py-1 text-violet-400">
                {indexedCount} indexed
              </span>
            </div>
          ) : null}
        </div>

        <div className="mt-3 rounded-xl border border-app-border bg-app-background p-3">
          <div className="mb-2 flex items-center justify-between">
            <div className="text-[11px] font-black uppercase tracking-widest text-app-muted">
              Local LLM
            </div>
            <button
              onClick={selectModel}
              className="text-app-accent hover:text-app-accent/80 transition-colors lowercase font-bold text-[10px]"
            >
              {providerSettings?.localModelPath ? '[change]' : '[select]'}
            </button>
          </div>
          <div className="mb-2 text-[11px] font-medium text-app-muted">
            {providerSettings?.localModelPath
              ? 'Model loaded for local Q&A and context-grounded answers.'
              : 'Select a local GGUF model to enable private on-device answers.'}
          </div>
          <div className="flex items-center gap-2 rounded-lg border border-app-border bg-app-panel p-2 text-[12px] font-medium text-app-text">
            <Brain
              className={`h-4 w-4 shrink-0 ${llmRunning ? 'text-emerald-400' : 'text-violet-400'}`}
            />
            <span className="truncate">
              {providerSettings?.localModelPath
                ? providerSettings.localModelPath.split(/[/\\]/).pop()
                : 'No model selected'}
            </span>
            <span
              className={`ml-auto h-3.5 w-3.5 rounded-full ${llmRunning ? 'bg-emerald-500 animate-pulse shadow-[0_0_10px_rgba(16,185,129,0.7)]' : 'bg-amber-400 shadow-[0_0_8px_rgba(251,191,36,0.45)]'}`}
              title={llmRunning ? 'Ready' : 'Not Ready'}
            />
          </div>
          <div className="mt-2 text-[11px] font-black uppercase tracking-widest">
            <span className={llmRunning ? 'text-emerald-400' : 'text-amber-500'}>
              {llmRunning ? 'Ready' : 'Not Ready'}
            </span>
          </div>
          <div className="mt-2">
            {llmRunning ? (
              <button
                onClick={stopServer}
                className="flex w-full items-center justify-center gap-2 rounded-lg bg-red-500/10 border border-red-500/30 py-2 text-[12px] font-black text-red-400 hover:bg-red-500/20 transition-all uppercase tracking-wider"
              >
                <Square className="h-3.5 w-3.5 fill-current" />
                Stop Server
              </button>
            ) : (
              <button
                onClick={startServer}
                disabled={isStartingServer || !providerSettings?.localModelPath}
                className="flex w-full items-center justify-center gap-2 rounded-lg bg-emerald-500/10 border border-emerald-500/30 py-2 text-[12px] font-black text-emerald-400 hover:bg-emerald-500/20 transition-all uppercase tracking-wider disabled:opacity-50"
              >
                <Play className="h-3.5 w-3.5 fill-current" />
                {isStartingServer ? 'Starting...' : 'Start Local Server'}
              </button>
            )}
          </div>
          {serverStatus ? (
            <div className="mt-2 text-[11px] font-medium text-app-muted">{serverStatus}</div>
          ) : null}
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
        <button
          className="border-b-2 border-transparent px-4 py-2.5 text-app-muted hover:text-app-text data-[active=true]:border-app-accent data-[active=true]:text-app-text transition-all"
          data-active={tab === 'sources'}
          type="button"
          onClick={() => setTab('sources')}
        >
          Sources
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
              <div className="mb-2 px-1">
                <label className="sr-only" htmlFor="explorer-search">
                  Search files
                </label>
                <div className="flex items-center gap-2 rounded-lg border border-app-border bg-app-background px-2.5 py-2">
                  <Search className="h-3.5 w-3.5 text-app-muted" aria-hidden="true" />
                  <input
                    id="explorer-search"
                    value={explorerQuery}
                    onChange={(event) => setExplorerQuery(event.target.value)}
                    placeholder="Search file path..."
                    className="w-full bg-transparent text-[12px] font-medium text-app-text outline-none placeholder:text-app-muted"
                  />
                </div>
              </div>
              <div className="space-y-1">
                {fileTree.map((node) => (
                  <TreeNodeRow
                    key={node.path}
                    node={node}
                    depth={0}
                    activeSourcePath={activeSourcePath}
                    expandedGroups={expandedGroups}
                    onToggleGroup={toggleGroup}
                    onSelectFile={(path) => {
                      setActiveSourcePath(path);
                      setActivePanel('graph');
                    }}
                  />
                ))}
                {fileTree.length === 0 && (
                  <div className="p-6 text-center text-app-muted font-medium">
                    {projectPath
                      ? isProjectLoading
                        ? 'Loading project and building wiki...'
                        : explorerQuery.trim()
                          ? 'No files match your search.'
                          : 'No indexed files yet.'
                      : 'Select a project folder to begin.'}
                  </div>
                )}
              </div>
            </div>
          ) : tab === 'wiki' ? (
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
          ) : (
            <div className="pb-4">
              <div className="px-2 py-1.5 text-[12px] font-black uppercase tracking-widest text-app-muted/70">
                SOURCES
              </div>
              <div className="space-y-1">
                {sourceItems.map((item) => (
                  <button
                    key={item.label}
                    className="flex min-h-9 w-full items-center gap-2.5 rounded px-2 text-left font-medium text-app-muted hover:bg-app-panelSoft hover:text-app-text transition-all"
                    type="button"
                    onClick={() => {
                      setActiveSourcePath(item.path);
                      setActivePanel('graph');
                    }}
                  >
                    <FileCode2 className="h-4 w-4 shrink-0 text-emerald-400" aria-hidden="true" />
                    <span className="truncate" title={item.label}>
                      {item.label}
                    </span>
                  </button>
                ))}
                {sourceItems.length === 0 && (
                  <div className="p-3 text-[12px] text-app-muted">
                    Ask Local Brain to populate citations and sources.
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Fixed Metrics Section - Sticky at bottom of scrollable area */}
        <div className="sticky bottom-0 shrink-0 border-t border-app-border p-4 bg-app-panel/95 backdrop-blur-md shadow-[0_-10px_30px_-15px_rgba(0,0,0,0.7)] z-10">
          <div className="mb-3 rounded-xl border border-app-border bg-app-background p-3">
            <div className="mb-2 flex items-center justify-between text-[11px] font-black uppercase tracking-widest text-app-muted">
              Theme
              <button
                type="button"
                onClick={toggleTheme}
                className={`rounded-md border px-2 py-1 text-[10px] font-black uppercase tracking-wider transition-colors ${
                  theme === 'dark'
                    ? 'border-indigo-500/40 bg-indigo-500/10 text-indigo-400'
                    : 'border-amber-500/40 bg-amber-500/10 text-amber-500'
                }`}
              >
                {theme === 'dark' ? 'Night Mode' : 'Day Mode'}
              </button>
            </div>
            <div className="text-[11px] font-medium text-app-muted">
              Shortcut: Cmd/Ctrl + Shift + T
            </div>
          </div>
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

function sourceLabel(path: string, startLine: number | null, endLine: number | null) {
  if (startLine && endLine) {
    return startLine === endLine ? `${path}:L${startLine}` : `${path}:L${startLine}-L${endLine}`;
  }
  return path;
}

function buildFileTree(paths: string[]): FileTreeNode[] {
  const root: FileTreeNode = {
    name: '',
    path: '',
    kind: 'folder',
    children: [],
  };

  for (const fullPath of paths) {
    const parts = fullPath.split('/').filter(Boolean);
    let cursor = root;
    let currentPath = '';
    for (let i = 0; i < parts.length; i += 1) {
      const part = parts[i];
      currentPath = currentPath ? `${currentPath}/${part}` : part;
      const isFile = i === parts.length - 1;
      let child = cursor.children.find(
        (item) => item.name === part && item.kind === (isFile ? 'file' : 'folder'),
      );
      if (!child) {
        child = {
          name: part,
          path: currentPath,
          kind: isFile ? 'file' : 'folder',
          color: isFile ? getFileColor(currentPath) : undefined,
          children: [],
        };
        cursor.children.push(child);
      }
      cursor = child;
    }
  }

  sortTreeChildren(root);
  return root.children;
}

function sortTreeChildren(node: FileTreeNode) {
  node.children.sort((left, right) => {
    if (left.kind !== right.kind) {
      return left.kind === 'folder' ? -1 : 1;
    }
    return left.name.localeCompare(right.name);
  });
  for (const child of node.children) {
    sortTreeChildren(child);
  }
}

function TreeNodeRow({
  node,
  depth,
  activeSourcePath,
  expandedGroups,
  onToggleGroup,
  onSelectFile,
}: {
  node: FileTreeNode;
  depth: number;
  activeSourcePath: string;
  expandedGroups: Set<string>;
  onToggleGroup: (path: string) => void;
  onSelectFile: (path: string) => void;
}) {
  if (node.kind === 'file') {
    return (
      <button
        className="flex min-h-8 w-full items-center gap-2 rounded px-2 text-left font-medium text-app-muted hover:bg-app-panelSoft hover:text-app-text data-[active=true]:bg-app-panelSoft data-[active=true]:text-app-text data-[active=true]:font-bold transition-all"
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
        data-active={node.path === activeSourcePath}
        type="button"
        onClick={() => onSelectFile(node.path)}
      >
        <FileCode2
          className={`h-4 w-4 shrink-0 ${node.color ?? 'text-app-muted'}`}
          aria-hidden="true"
        />
        <span className="truncate" title={node.name}>
          {node.name}
        </span>
      </button>
    );
  }

  const isExpanded = expandedGroups.has(node.path);

  return (
    <div>
      <button
        className="flex w-full items-center gap-2 rounded px-2 py-1.5 font-bold text-app-text/90 hover:bg-app-panelSoft transition-colors"
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
        type="button"
        onClick={() => onToggleGroup(node.path)}
      >
        <ChevronRight
          className={`h-4 w-4 transition-transform ${isExpanded ? 'rotate-90' : ''}`}
          aria-hidden="true"
        />
        <Folder className="h-4 w-4 text-app-accent" aria-hidden="true" />
        <span className="truncate" title={node.name}>
          {node.name}
        </span>
      </button>
      {isExpanded && (
        <div className="space-y-0.5">
          {node.children.map((child) => (
            <TreeNodeRow
              key={child.path}
              node={child}
              depth={depth + 1}
              activeSourcePath={activeSourcePath}
              expandedGroups={expandedGroups}
              onToggleGroup={onToggleGroup}
              onSelectFile={onSelectFile}
            />
          ))}
        </div>
      )}
    </div>
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
