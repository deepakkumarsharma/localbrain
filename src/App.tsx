import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { confirm } from '@tauri-apps/plugin-dialog';
import { useEffect, useState, useCallback, useRef } from 'react';
import { ErrorBanner } from './components/ErrorBanner';
import { MainPanel } from './components/MainPanel';
import { RightPanel } from './components/RightPanel';
import { Sidebar } from './components/Sidebar';
import { SplashScreen } from './components/SplashScreen';
import { initFileWatcher } from './lib/fileWatcher';
import { detectDatabaseStructure } from './lib/database';
import type { DatabaseSchema } from './lib/database';
import { indexPath, resolveProjectRoot, setWorkspaceRoot } from './lib/indexer';
import type { IndexPathSummary, IndexProgressEvent } from './lib/indexer';
import { clearSearchIndex, rebuildSearchIndex } from './lib/search';
import type { SearchIndexSummary } from './lib/search';
import { generate_wiki } from './lib/wiki';
import type { WikiSummary } from './lib/wiki';
import { getProviderSettings, setLastProjectPath } from './lib/settings';
import { useAppStore } from './store/useAppStore';

const DEFAULT_STEP_TIMEOUT_MS = 120_000;
const INDEXING_TIMEOUT_MS = 1_800_000;
const SEARCH_REBUILD_TIMEOUT_MS = 600_000;
const WIKI_TIMEOUT_MS = 600_000;

interface ProjectSnapshot {
  indexPathSummary: IndexPathSummary;
  searchIndexSummary: SearchIndexSummary;
  wikiSummary: WikiSummary;
  databaseSchema: DatabaseSchema | null;
  databaseViewEnabled: boolean;
}

async function withTimeout<T>(
  promise: Promise<T>,
  label: string,
  timeoutMs = DEFAULT_STEP_TIMEOUT_MS,
) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  const timeoutPromise = new Promise<T>((_, reject) => {
    timer = setTimeout(
      () => reject(new Error(`${label} timed out after ${timeoutMs / 1000}s`)),
      timeoutMs,
    );
  });

  try {
    return await Promise.race([promise, timeoutPromise]);
  } finally {
    if (timer) {
      clearTimeout(timer);
    }
  }
}

function indexingStatus(progress: IndexProgressEvent) {
  if (progress.filesTotal === 0) {
    return '20% · Discovering indexable files...';
  }

  const ratio = Math.min(progress.filesSeen / progress.filesTotal, 1);
  const percent = Math.min(54, 20 + Math.floor(ratio * 34));
  const currentFile = progress.currentPath?.split('/').pop();
  const suffix = currentFile ? ` · ${currentFile}` : '';
  return `${percent}% · Indexed ${progress.filesSeen}/${progress.filesTotal} files${suffix}`;
}

export default function App() {
  const {
    setAppVersion,
    theme,
    toggleTheme,
    projectPath,
    setIndexPathResult,
    setIndexProgress,
    setIndexError,
    setWikiError,
    setWikiResult,
    setSearchIndexResult,
    setSearchError,
    setProjectPath,
    setProjectLoading,
    setDatabaseSchema,
    setDatabaseViewEnabled,
    clearProjectData,
  } = useAppStore();
  const [sidebarWidth, setSidebarWidth] = useState(400);
  const [rightPanelWidth, setRightPanelWidth] = useState(600);
  const [showSplash, setShowSplash] = useState(true);
  const isResizingSidebar = useRef(false);
  const isResizingRightPanel = useRef(false);
  const watcherUnlistenRef = useRef<(() => void) | null>(null);
  const projectLoadRunRef = useRef(0);
  const lastProgressUpdateRef = useRef({ at: 0, filesSeen: -1 });
  const projectSnapshotCacheRef = useRef<Map<string, ProjectSnapshot>>(new Map());
  const hasHydratedProjectPathRef = useRef(false);

  useEffect(() => {
    void invoke<string>('get_app_version')
      .then(setAppVersion)
      .catch(() => setAppVersion('unknown'));
  }, [setAppVersion]);

  useEffect(() => {
    return () => {
      if (watcherUnlistenRef.current) {
        watcherUnlistenRef.current();
      }
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    void listen<IndexProgressEvent>('index-progress', (event) => {
      const progress = event.payload;
      if (progress.runId !== projectLoadRunRef.current) {
        return;
      }
      if (!useAppStore.getState().isProjectLoading) {
        return;
      }
      const now = Date.now();
      const isComplete =
        progress.phase === 'complete' ||
        (progress.filesTotal > 0 && progress.filesSeen >= progress.filesTotal);
      const shouldThrottle =
        !isComplete &&
        now - lastProgressUpdateRef.current.at < 140 &&
        progress.filesSeen !== lastProgressUpdateRef.current.filesSeen;

      if (shouldThrottle) {
        return;
      }

      lastProgressUpdateRef.current = { at: now, filesSeen: progress.filesSeen };
      setIndexProgress(progress);
      setProjectLoading(true, indexingStatus(progress));
    }).then((cleanup) => {
      if (disposed) {
        cleanup();
        return;
      }
      unlisten = cleanup;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [setIndexProgress, setProjectLoading]);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    if (!hasHydratedProjectPathRef.current) {
      return;
    }
    void setLastProjectPath(projectPath).catch(() => {});
  }, [projectPath]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.repeat) {
        return;
      }

      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 't') {
        event.preventDefault();
        toggleTheme();
      }
    }

    window.addEventListener('keydown', handleKeyDown);

    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleTheme]);

  const startResizingSidebar = useCallback(() => {
    isResizingSidebar.current = true;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, []);

  const startResizingRightPanel = useCallback(() => {
    isResizingRightPanel.current = true;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, []);

  const stopResizing = useCallback(() => {
    isResizingSidebar.current = false;
    isResizingRightPanel.current = false;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  const resize = useCallback((e: MouseEvent) => {
    if (isResizingSidebar.current) {
      const newWidth = Math.min(Math.max(200, e.clientX), 400);
      setSidebarWidth(newWidth);
    } else if (isResizingRightPanel.current) {
      const newWidth = Math.min(Math.max(400, window.innerWidth - e.clientX), 1000);
      setRightPanelWidth(newWidth);
    }
  }, []);

  useEffect(() => {
    window.addEventListener('mousemove', resize);
    window.addEventListener('mouseup', stopResizing);
    return () => {
      window.removeEventListener('mousemove', resize);
      window.removeEventListener('mouseup', stopResizing);
    };
  }, [resize, stopResizing]);

  const loadProject = useCallback(
    async (selectedPath: string) => {
      const runId = projectLoadRunRef.current + 1;
      projectLoadRunRef.current = runId;
      lastProgressUpdateRef.current = { at: 0, filesSeen: -1 };
      const isCurrentRun = () => projectLoadRunRef.current === runId;
      if (watcherUnlistenRef.current) {
        watcherUnlistenRef.current();
        watcherUnlistenRef.current = null;
      }
      setProjectLoading(true, '5% · Preparing workspace...');
      setIndexError(null);
      setWikiError(null);
      setSearchError(null);

      try {
        const path = await withTimeout(resolveProjectRoot(selectedPath), 'Project root detection');
        if (!isCurrentRun()) return;

        const cachedSnapshot = projectSnapshotCacheRef.current.get(path);
        if (cachedSnapshot) {
          clearProjectData();
          setIndexProgress(null);
          setProjectPath(path);
          setIndexPathResult(cachedSnapshot.indexPathSummary);
          setSearchIndexResult(cachedSnapshot.searchIndexSummary);
          setWikiResult(cachedSnapshot.wikiSummary);
          setDatabaseSchema(cachedSnapshot.databaseSchema);
          setDatabaseViewEnabled(cachedSnapshot.databaseViewEnabled);

          watcherUnlistenRef.current = await withTimeout(
            initFileWatcher(path),
            'File watcher startup',
          );
          if (!isCurrentRun()) return;

          setProjectLoading(false, '100% · Ready (restored from local session cache)');
          return;
        }

        clearProjectData();
        setIndexProgress(null);
        setProjectPath(path);
        setDatabaseSchema(null);
        setDatabaseViewEnabled(false);
        if (path !== selectedPath) {
          setProjectLoading(true, `10% · Detected project root: ${path}`);
        }
        await withTimeout(setWorkspaceRoot(path), 'Workspace initialization');
        if (!isCurrentRun()) return;
        setProjectLoading(
          true,
          '20% · Scanning source files (large repos may take a few minutes)...',
        );
        const summary = await withTimeout(
          indexPath(path, runId),
          'Source indexing',
          INDEXING_TIMEOUT_MS,
        );
        if (!isCurrentRun()) return;
        setIndexPathResult(summary);
        setIndexProgress(null);

        setProjectLoading(true, '55% · Rebuilding search index...');
        await withTimeout(clearSearchIndex(), 'Search index reset');
        if (!isCurrentRun()) return;
        const searchSummary = await withTimeout(
          rebuildSearchIndex(path),
          'Search index rebuild',
          SEARCH_REBUILD_TIMEOUT_MS,
        );
        if (!isCurrentRun()) return;
        setSearchIndexResult(searchSummary);

        setProjectLoading(true, '75% · Starting file watcher...');
        watcherUnlistenRef.current = await withTimeout(
          initFileWatcher(path),
          'File watcher startup',
        );
        if (!isCurrentRun()) return;

        setProjectLoading(true, '88% · Generating wiki from indexed sources...');
        const wikiSummary = await withTimeout(
          generate_wiki(path),
          'Wiki generation',
          WIKI_TIMEOUT_MS,
        );
        if (!isCurrentRun()) return;
        setWikiResult(wikiSummary);
        let detectedDatabaseSchema: DatabaseSchema | null = null;
        let databaseViewEnabled = false;
        try {
          detectedDatabaseSchema = await withTimeout(
            detectDatabaseStructure(path),
            'Database structure detection',
            60_000,
          );
        } catch {
          detectedDatabaseSchema = null;
        }
        if (!isCurrentRun()) return;
        setDatabaseSchema(detectedDatabaseSchema);
        setProjectLoading(
          false,
          `100% · Ready: ${summary.filesSeen} indexed · ${summary.filesSkipped} skipped · ${summary.errors.length} errors`,
        );
        if (detectedDatabaseSchema && isCurrentRun()) {
          const shouldEnableDatabaseView = await confirm('Database detected. Add Database view?');
          if (!isCurrentRun()) return;
          databaseViewEnabled = shouldEnableDatabaseView;
        }
        if (isCurrentRun()) {
          setDatabaseViewEnabled(databaseViewEnabled);
          projectSnapshotCacheRef.current.set(path, {
            indexPathSummary: summary,
            searchIndexSummary: searchSummary,
            wikiSummary,
            databaseSchema: detectedDatabaseSchema,
            databaseViewEnabled,
          });
        }
        if (summary.errors.length > 0) {
          setIndexError(summary.errors.join('\n'));
        }
      } catch (error) {
        if (!isCurrentRun()) return;
        const message = error instanceof Error ? error.message : String(error);
        setProjectLoading(false, 'Indexing failed');
        setIndexError(message);
      }
    },
    [
      clearProjectData,
      setIndexError,
      setIndexPathResult,
      setIndexProgress,
      setProjectLoading,
      setProjectPath,
      setDatabaseSchema,
      setDatabaseViewEnabled,
      setSearchError,
      setSearchIndexResult,
      setWikiError,
      setWikiResult,
    ],
  );

  const removeProject = useCallback(() => {
    projectLoadRunRef.current += 1;
    lastProgressUpdateRef.current = { at: 0, filesSeen: -1 };
    if (watcherUnlistenRef.current) {
      watcherUnlistenRef.current();
      watcherUnlistenRef.current = null;
    }
    setProjectPath(null);
    clearProjectData();
    setIndexProgress(null);
    setProjectLoading(false, 'No project selected');
  }, [clearProjectData, setIndexProgress, setProjectLoading, setProjectPath]);

  useEffect(() => {
    void getProviderSettings()
      .then((settings) => {
        hasHydratedProjectPathRef.current = true;
        if (settings.lastProjectPath) {
          return loadProject(settings.lastProjectPath);
        }
        return undefined;
      })
      .catch(() => {
        hasHydratedProjectPathRef.current = true;
      });
  }, [loadProject]);

  if (showSplash) {
    return <SplashScreen onComplete={() => setShowSplash(false)} />;
  }

  return (
    <div className="flex h-screen min-w-[1180px] flex-col overflow-hidden bg-app-background text-app-text">
      <ErrorBanner />
      <div
        className="grid min-h-0 flex-1"
        style={{
          gridTemplateColumns: `${sidebarWidth}px minmax(0, 1fr) ${rightPanelWidth}px`,
        }}
      >
        <div className="relative h-full overflow-hidden">
          <Sidebar onSelectProject={loadProject} onRemoveProject={removeProject} />
          <div
            className="absolute right-0 top-0 z-50 h-full w-1.5 cursor-col-resize hover:bg-app-accent/30 active:bg-app-accent transition-colors"
            onMouseDown={startResizingSidebar}
          />
        </div>
        <MainPanel />
        <div className="relative h-full overflow-hidden">
          <div
            className="absolute left-0 top-0 z-50 h-full w-1.5 cursor-col-resize hover:bg-app-accent/30 active:bg-app-accent transition-colors"
            onMouseDown={startResizingRightPanel}
          />
          <RightPanel />
        </div>
      </div>
    </div>
  );
}
