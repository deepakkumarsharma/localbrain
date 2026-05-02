import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MainPanel } from './components/MainPanel';
import { RightPanel } from './components/RightPanel';
import { Sidebar } from './components/Sidebar';
import { useAppStore } from './store/useAppStore';

export default function App() {
  const { setAppVersion } = useAppStore();

  useEffect(() => {
    void invoke<string>('get_app_version')
      .then(setAppVersion)
      .catch(() => setAppVersion('unknown'));
  }, [setAppVersion]);

  return (
    <div className="grid h-screen min-w-[1024px] grid-cols-[240px_minmax(0,1fr)_350px] overflow-hidden bg-app-background text-app-text">
      <Sidebar />
      <MainPanel />
      <RightPanel />
    </div>
  );
}
