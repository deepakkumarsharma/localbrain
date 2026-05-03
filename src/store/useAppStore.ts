import { create } from 'zustand';
import type { GraphIngestSummary } from '../lib/graph';
import type { ParsedFile } from '../lib/parser';

type ActivePanel = 'chat' | 'graph';
type Theme = 'dark' | 'light';

interface AppState {
  activePanel: ActivePanel;
  appVersion: string;
  theme: Theme;
  lastFileChange: string | null;
  parsedFile: ParsedFile | null;
  parserError: string | null;
  graphSummary: GraphIngestSummary | null;
  graphSymbols: ParsedFile['symbols'];
  graphError: string | null;
  setActivePanel: (panel: ActivePanel) => void;
  setAppVersion: (version: string) => void;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  setLastFileChange: (path: string) => void;
  setParsedFile: (parsedFile: ParsedFile) => void;
  setParserError: (error: string | null) => void;
  setGraphResult: (summary: GraphIngestSummary, symbols: ParsedFile['symbols']) => void;
  setGraphError: (error: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  activePanel: 'chat',
  appVersion: 'loading',
  theme: 'dark',
  lastFileChange: null,
  parsedFile: null,
  parserError: null,
  graphSummary: null,
  graphSymbols: [],
  graphError: null,
  setActivePanel: (panel) => set({ activePanel: panel }),
  setAppVersion: (version) => set({ appVersion: version }),
  setTheme: (theme) => set({ theme }),
  toggleTheme: () => set((state) => ({ theme: state.theme === 'dark' ? 'light' : 'dark' })),
  setLastFileChange: (path) => set({ lastFileChange: path }),
  setParsedFile: (parsedFile) => set({ parsedFile, parserError: null }),
  setParserError: (error) => set({ parserError: error }),
  setGraphResult: (summary, symbols) =>
    set({ graphSummary: summary, graphSymbols: symbols, graphError: null }),
  setGraphError: (error) => set({ graphError: error }),
}));
