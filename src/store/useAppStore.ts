import { create } from 'zustand';
import type { GraphIngestSummary } from '../lib/graph';
import type { IndexFileSummary, IndexPathSummary, IndexRunSummary } from '../lib/indexer';
import type { FileMetadata } from '../lib/metadata';
import type { ParsedFile } from '../lib/parser';

type ActivePanel = 'chat' | 'graph';
type Theme = 'dark' | 'light';

interface AppState {
  activePanel: ActivePanel;
  appVersion: string;
  theme: Theme;
  lastFileChange: string | null;
  lastFileChangeAt: number | null;
  parsedFile: ParsedFile | null;
  parserError: string | null;
  graphSummary: GraphIngestSummary | null;
  graphSymbols: ParsedFile['symbols'];
  graphError: string | null;
  metadata: FileMetadata | null;
  metadataError: string | null;
  indexFileSummary: IndexFileSummary | null;
  indexPathSummary: IndexPathSummary | null;
  indexRun: IndexRunSummary | null;
  indexError: string | null;
  setActivePanel: (panel: ActivePanel) => void;
  setAppVersion: (version: string) => void;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  setLastFileChange: (path: string) => void;
  setParsedFile: (parsedFile: ParsedFile) => void;
  setParserError: (error: string | null) => void;
  setGraphResult: (summary: GraphIngestSummary, symbols: ParsedFile['symbols']) => void;
  setGraphError: (error: string | null) => void;
  setMetadataResult: (metadata: FileMetadata) => void;
  setMetadataError: (error: string | null) => void;
  setIndexFileResult: (summary: IndexFileSummary) => void;
  setIndexPathResult: (summary: IndexPathSummary) => void;
  setIndexRun: (run: IndexRunSummary | null) => void;
  setIndexError: (error: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  activePanel: 'chat',
  appVersion: 'loading',
  theme: 'dark',
  lastFileChange: null,
  lastFileChangeAt: null,
  parsedFile: null,
  parserError: null,
  graphSummary: null,
  graphSymbols: [],
  graphError: null,
  metadata: null,
  metadataError: null,
  indexFileSummary: null,
  indexPathSummary: null,
  indexRun: null,
  indexError: null,
  setActivePanel: (panel) => set({ activePanel: panel }),
  setAppVersion: (version) => set({ appVersion: version }),
  setTheme: (theme) => set({ theme }),
  toggleTheme: () => set((state) => ({ theme: state.theme === 'dark' ? 'light' : 'dark' })),
  setLastFileChange: (path) => set({ lastFileChange: path, lastFileChangeAt: Date.now() }),
  setParsedFile: (parsedFile) => set({ parsedFile, parserError: null }),
  setParserError: (error) => set({ parserError: error }),
  setGraphResult: (summary, symbols) =>
    set({ graphSummary: summary, graphSymbols: symbols, graphError: null }),
  setGraphError: (error) => set({ graphError: error }),
  setMetadataResult: (metadata) => set({ metadata, metadataError: null }),
  setMetadataError: (error) => set({ metadataError: error }),
  setIndexFileResult: (summary) =>
    set({
      indexFileSummary: summary,
      metadata: summary.metadata,
      graphSummary: summary.graph,
      graphSymbols: [],
      indexError: null,
      metadataError: null,
      graphError: null,
    }),
  setIndexPathResult: (summary) =>
    set({
      indexPathSummary: summary,
      indexRun: summary.run,
      indexError: summary.errors.length > 0 ? summary.errors.join('\n') : null,
    }),
  setIndexRun: (run) => set({ indexRun: run }),
  setIndexError: (error) => set({ indexError: error }),
}));
