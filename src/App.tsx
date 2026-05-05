import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState, useCallback, useRef } from 'react';
import { MainPanel } from './components/MainPanel';
import { RightPanel } from './components/RightPanel';
import { Sidebar } from './components/Sidebar';
import { initFileWatcher } from './lib/fileWatcher';
import { indexPath } from './lib/indexer';
import { useAppStore } from './store/useAppStore';

export default function App() {
  const { setAppVersion, theme, toggleTheme, setIndexPathResult } = useAppStore();
  const [sidebarWidth, setSidebarWidth] = useState(400);
  const [rightPanelWidth, setRightPanelWidth] = useState(450);
  const isResizingSidebar = useRef(false);
  const isResizingRightPanel = useRef(false);

  useEffect(() => {
    void invoke<string>('get_app_version')
      .then(setAppVersion)
      .catch(() => setAppVersion('unknown'));

    void indexPath('.')
      .then(setIndexPathResult)
      .catch((error) => console.error('Initial index failed:', error));
  }, [setAppVersion, setIndexPathResult]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void initFileWatcher('.')
      .then((ul) => {
        unlisten = ul;
      })
      .catch(console.error);

    return () => {
      if (unlisten) {
        unlisten();
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
      const newWidth = Math.min(Math.max(300, e.clientX), 500);
      setSidebarWidth(newWidth);
    } else if (isResizingRightPanel.current) {
      const newWidth = Math.min(Math.max(300, window.innerWidth - e.clientX), 600);
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

  return (
    <div
      className="grid h-screen min-w-[1180px] overflow-hidden bg-app-background text-app-text"
      style={{
        gridTemplateColumns: `${sidebarWidth}px minmax(0, 1fr) ${rightPanelWidth}px`,
      }}
    >
      <div className="relative h-full overflow-hidden">
        <Sidebar />
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
  );
}
