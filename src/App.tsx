import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState, useCallback, useRef } from 'react';
import { ErrorBanner } from './components/ErrorBanner';
import { MainPanel } from './components/MainPanel';
import { RightPanel } from './components/RightPanel';
import { Sidebar } from './components/Sidebar';
import { initFileWatcher } from './lib/fileWatcher';
import { indexPath, resolveProjectRoot, setWorkspaceRoot } from './lib/indexer';
import { clearSearchIndex, rebuildSearchIndex } from './lib/search';
import { generate_wiki } from './lib/wiki';
import { useAppStore } from './store/useAppStore';

export default function App() {
  const {
    setAppVersion,
    theme,
    toggleTheme,
    setIndexPathResult,
    setIndexError,
    setWikiError,
    setWikiResult,
    setSearchIndexResult,
    setSearchError,
    setProjectPath,
    setProjectLoading,
    clearProjectData,
  } = useAppStore();
  const [sidebarWidth, setSidebarWidth] = useState(400);
  const [rightPanelWidth, setRightPanelWidth] = useState(450);
  const isResizingSidebar = useRef(false);
  const isResizingRightPanel = useRef(false);
  const watcherUnlistenRef = useRef<(() => void) | null>(null);

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
    document.documentElement.dataset.theme = theme;
  }, [theme]);

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
      const newWidth = Math.min(Math.max(250, window.innerWidth - e.clientX), 400);
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
      clearProjectData();
      setProjectLoading(true, 'Preparing workspace...');
      setIndexError(null);
      setWikiError(null);
      setSearchError(null);

      try {
        const path = await resolveProjectRoot(selectedPath);
        setProjectPath(path);
        if (path !== selectedPath) {
          setProjectLoading(true, `Detected project root: ${path}`);
        }
        await setWorkspaceRoot(path);
        setProjectLoading(true, 'Scanning source files...');
        const summary = await indexPath(path);
        setIndexPathResult(summary);

        setProjectLoading(true, 'Rebuilding search index...');
        await clearSearchIndex();
        const searchSummary = await rebuildSearchIndex(path);
        setSearchIndexResult(searchSummary);

        setProjectLoading(true, 'Starting file watcher...');
        if (watcherUnlistenRef.current) {
          watcherUnlistenRef.current();
        }
        watcherUnlistenRef.current = await initFileWatcher(path);

        setProjectLoading(true, 'Generating wiki from indexed sources...');
        const wikiSummary = await generate_wiki(path);
        setWikiResult(wikiSummary);
        if (summary.errors.length > 0) {
          setIndexError(summary.errors.join('\n'));
        }
        setProjectLoading(
          false,
          `Ready: ${summary.filesSeen} indexed · ${summary.filesSkipped} skipped · ${summary.errors.length} errors`,
        );
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setProjectLoading(false, 'Indexing failed');
        setIndexError(message);
      }
    },
    [
      clearProjectData,
      setIndexError,
      setIndexPathResult,
      setProjectLoading,
      setProjectPath,
      setSearchError,
      setSearchIndexResult,
      setWikiError,
      setWikiResult,
    ],
  );

  const removeProject = useCallback(() => {
    if (watcherUnlistenRef.current) {
      watcherUnlistenRef.current();
      watcherUnlistenRef.current = null;
    }
    setProjectPath(null);
    clearProjectData();
    setProjectLoading(false, 'No project selected');
  }, [clearProjectData, setProjectLoading, setProjectPath]);

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
