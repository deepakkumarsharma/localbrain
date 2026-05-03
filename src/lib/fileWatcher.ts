import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from '../store/useAppStore';

export async function initFileWatcher(projectPath: string) {
  const setLastFileChange = useAppStore.getState().setLastFileChange;

  try {
    await invoke('start_watcher', { path: projectPath });

    const unlisten = await listen<string>('file-changed', (event) => {
      setLastFileChange(event.payload);
    });

    return unlisten;
  } catch (error) {
    console.error('Failed to initialize watcher:', error);
    throw error;
  }
}
