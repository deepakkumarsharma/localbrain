import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { indexFile } from './indexer';
import { useAppStore } from '../store/useAppStore';
import { parseSourceFile } from './parser';
import { getGraphSymbols } from './graph';

// The backend watcher emits several useful file types, but incremental graph indexing is JS/TS-only.
const SOURCE_FILE_PATTERN = /\.(jsx?|tsx?)$/i;

export async function initFileWatcher(projectPath: string) {
  console.log('Initializing file watcher for:', projectPath);
  try {
    await invoke('start_watcher', { path: projectPath });

    const unlisten = await listen<string>('file-changed', async (event) => {
      const store = useAppStore.getState();
      console.log('File changed event received:', event.payload);
      const filePath = event.payload;

      store.setLastFileChange(filePath);

      if (!SOURCE_FILE_PATTERN.test(filePath)) {
        return;
      }

      // Small delay to allow editor to finish writing/unlocking the file
      await new Promise((resolve) => setTimeout(resolve, 50));

      try {
        // Clear previous error before trying
        store.setIndexError(null);

        // 1. Run incremental indexing (Metadata + KuzuDB)
        const indexResult = await indexFile(filePath);
        store.setIndexFileResult(indexResult);

        // 2. If the file being parsed in the UI is the one that changed, update it
        if (store.parsedFile?.path === filePath || filePath.endsWith('App.tsx')) {
          const parsed = await parseSourceFile(filePath);
          store.setParsedFile(parsed);

          // Also update graph symbols view if applicable
          if (indexResult.graph) {
            const symbols = await getGraphSymbols(filePath);
            store.setGraphResult(indexResult.graph, symbols);
          }
        }
      } catch (error: unknown) {
        console.error('Watcher processing error:', error);
        store.setIndexError(error instanceof Error ? error.message : String(error));
      }
    });

    return unlisten;
  } catch (error) {
    console.error('Failed to initialize watcher:', error);
    throw error;
  }
}
