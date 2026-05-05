import { invoke } from '@tauri-apps/api/core';
import { useEffect } from 'react';
import { MainPanel } from './components/MainPanel';
import { RightPanel } from './components/RightPanel';
import { Sidebar } from './components/Sidebar';
import { initFileWatcher } from './lib/fileWatcher';
import { useAppStore } from './store/useAppStore';

export default function App() {
  const { setAppVersion, theme, toggleTheme } = useAppStore();

  useEffect(() => {
    void invoke<string>('get_app_version')
      .then(setAppVersion)
      .catch(() => setAppVersion('unknown'));
  }, [setAppVersion]);

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

  return (
    <div className="grid h-screen min-w-[1180px] grid-cols-[260px_minmax(0,1fr)_320px] overflow-hidden bg-app-background text-app-text">
      <Sidebar />
      <MainPanel />
      <RightPanel />
    </div>
  );
}
