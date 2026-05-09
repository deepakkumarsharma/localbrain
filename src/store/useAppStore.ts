import { create } from 'zustand';
import type { GraphContext, GraphIngestSummary, GraphViewData, GraphViewNode } from '../lib/graph';
import type {
  IndexFileSummary,
  IndexPathSummary,
  IndexProgressEvent,
  IndexRunSummary,
} from '../lib/indexer';
import type { FileMetadata } from '../lib/metadata';
import type { ParsedFile } from '../lib/parser';
import type { SearchIndexSummary, SearchResult } from '../lib/search';
import type { ChatMessage, Citation } from '../lib/chat';
import type { AgentApiStatus } from '../lib/api';
import type { DatabaseSchema } from '../lib/database';
import type { ProviderSettings } from '../lib/settings';
import type { WikiSummary } from '../lib/wiki';

type ActivePanel = 'graph' | 'wiki' | 'flow' | 'database';
type Theme = 'dark' | 'light';

function getSystemTheme(): Theme {
  if (typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    return 'dark';
  }
  return 'light';
}

interface AppState {
  activePanel: ActivePanel;
  appVersion: string;
  theme: Theme;
  activeSourcePath: string;
  lastFileChange: string | null;
  lastFileChangeAt: number | null;
  parsedFile: ParsedFile | null;
  parserError: string | null;
  graphSummary: GraphIngestSummary | null;
  graphSymbols: ParsedFile['symbols'];
  graphContext: GraphContext[];
  graphView: GraphViewData | null;
  selectedGraphNode: GraphViewNode | null;
  graphError: string | null;
  metadata: FileMetadata | null;
  metadataError: string | null;
  indexFileSummary: IndexFileSummary | null;
  indexPathSummary: IndexPathSummary | null;
  indexRun: IndexRunSummary | null;
  indexProgress: IndexProgressEvent | null;
  indexError: string | null;
  wikiSummary: WikiSummary | null;
  wikiError: string | null;
  searchIndexSummary: SearchIndexSummary | null;
  searchResults: SearchResult[];
  searchQuery: string;
  searchError: string | null;
  chatMessages: ChatMessage[];
  chatError: string | null;
  citations: Citation[];
  providerSettings: ProviderSettings | null;
  agentApiStatus: AgentApiStatus | null;
  llmRunning: boolean;
  projectPath: string | null;
  isProjectLoading: boolean;
  projectStatus: string | null;
  databaseSchema: DatabaseSchema | null;
  databaseViewEnabled: boolean;
  setActivePanel: (panel: ActivePanel) => void;
  setAppVersion: (version: string) => void;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  setActiveSourcePath: (path: string) => void;
  setLastFileChange: (path: string) => void;
  setParsedFile: (parsedFile: ParsedFile) => void;
  setParserError: (error: string | null) => void;
  setGraphResult: (summary: GraphIngestSummary, symbols: ParsedFile['symbols']) => void;
  setGraphContext: (context: GraphContext[]) => void;
  setGraphView: (view: GraphViewData) => void;
  setSelectedGraphNode: (node: GraphViewNode | null) => void;
  setGraphError: (error: string | null) => void;
  setMetadataResult: (metadata: FileMetadata) => void;
  setMetadataError: (error: string | null) => void;
  setIndexFileResult: (summary: IndexFileSummary) => void;
  setIndexPathResult: (summary: IndexPathSummary) => void;
  setIndexRun: (run: IndexRunSummary | null) => void;
  setIndexProgress: (progress: IndexProgressEvent | null) => void;
  setIndexError: (error: string | null) => void;
  setWikiResult: (summary: WikiSummary) => void;
  setWikiError: (error: string | null) => void;
  setSearchIndexResult: (summary: SearchIndexSummary) => void;
  setSearchResults: (query: string, results: SearchResult[]) => void;
  setSearchError: (error: string | null) => void;
  addChatMessage: (message: ChatMessage) => void;
  replaceChatMessage: (id: string, message: ChatMessage) => void;
  setChatError: (error: string | null) => void;
  setCitations: (citations: Citation[]) => void;
  setProviderSettings: (settings: ProviderSettings) => void;
  setAgentApiStatus: (status: AgentApiStatus) => void;
  setLlmRunning: (running: boolean) => void;
  setProjectPath: (path: string | null) => void;
  setProjectLoading: (loading: boolean, status?: string | null) => void;
  setDatabaseSchema: (schema: DatabaseSchema | null) => void;
  setDatabaseViewEnabled: (enabled: boolean) => void;
  clearProjectData: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  activePanel: 'flow',
  appVersion: 'loading',
  theme: getSystemTheme(),
  activeSourcePath: '',
  lastFileChange: null,
  lastFileChangeAt: null,
  parsedFile: null,
  parserError: null,
  graphSummary: null,
  graphSymbols: [],
  graphContext: [],
  graphView: null,
  selectedGraphNode: null,
  graphError: null,
  metadata: null,
  metadataError: null,
  indexFileSummary: null,
  indexPathSummary: null,
  indexRun: null,
  indexProgress: null,
  indexError: null,
  wikiSummary: null,
  wikiError: null,
  searchIndexSummary: null,
  searchResults: [],
  searchQuery: '',
  searchError: null,
  chatMessages: [],
  chatError: null,
  citations: [],
  providerSettings: null,
  agentApiStatus: null,
  llmRunning: false,
  projectPath: null,
  isProjectLoading: false,
  projectStatus: null,
  databaseSchema: null,
  databaseViewEnabled: false,
  setActivePanel: (panel) => set({ activePanel: panel }),
  setAppVersion: (version) => set({ appVersion: version }),
  setTheme: (theme) => set({ theme }),
  toggleTheme: () => set((state) => ({ theme: state.theme === 'dark' ? 'light' : 'dark' })),
  setActiveSourcePath: (path) => set({ activeSourcePath: path }),
  setLastFileChange: (path) => set({ lastFileChange: path, lastFileChangeAt: Date.now() }),
  setParsedFile: (parsedFile) => set({ parsedFile, parserError: null }),
  setParserError: (error) => set({ parserError: error }),
  setGraphResult: (summary, symbols) =>
    set({ graphSummary: summary, graphSymbols: symbols, graphError: null }),
  setGraphContext: (context) => set({ graphContext: context, graphError: null }),
  setGraphView: (view) => set({ graphView: view, graphError: null }),
  setSelectedGraphNode: (node) => set({ selectedGraphNode: node }),
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
    set((state) => ({
      indexPathSummary: summary,
      indexRun: summary.run,
      indexError: summary.errors.length > 0 ? summary.errors.join('\n') : null,
      activeSourcePath: state.activeSourcePath || summary.files?.[0]?.path || '',
    })),
  setIndexRun: (run) => set({ indexRun: run }),
  setIndexProgress: (progress) => set({ indexProgress: progress }),
  setIndexError: (error) => set({ indexError: error }),
  setWikiResult: (summary) =>
    set({
      wikiSummary: summary,
      wikiError: summary.errors.length > 0 ? summary.errors.join('\n') : null,
    }),
  setWikiError: (error) => set({ wikiError: error }),
  setSearchIndexResult: (summary) =>
    set({
      searchIndexSummary: summary,
      searchError: summary.errors.length > 0 ? summary.errors.join('\n') : null,
    }),
  setSearchResults: (query, results) =>
    set({ searchQuery: query, searchResults: results, searchError: null }),
  setSearchError: (error) => set({ searchError: error }),
  addChatMessage: (message) => set((state) => ({ chatMessages: [...state.chatMessages, message] })),
  replaceChatMessage: (id, message) =>
    set((state) => ({
      chatMessages: state.chatMessages.map((existing) => (existing.id === id ? message : existing)),
    })),
  setChatError: (error) => set({ chatError: error }),
  setCitations: (citations) => set({ citations }),
  setProviderSettings: (settings) => set({ providerSettings: settings }),
  setAgentApiStatus: (status) => set({ agentApiStatus: status }),
  setLlmRunning: (running) => set({ llmRunning: running }),
  setProjectPath: (path) => set({ projectPath: path }),
  setProjectLoading: (loading, status = null) =>
    set({ isProjectLoading: loading, projectStatus: status }),
  setDatabaseSchema: (schema) => set({ databaseSchema: schema }),
  setDatabaseViewEnabled: (enabled) => set({ databaseViewEnabled: enabled }),
  clearProjectData: () =>
    set({
      activeSourcePath: '',
      parsedFile: null,
      parserError: null,
      graphSummary: null,
      graphSymbols: [],
      graphContext: [],
      graphView: null,
      selectedGraphNode: null,
      graphError: null,
      metadata: null,
      metadataError: null,
      indexFileSummary: null,
      indexPathSummary: null,
      indexRun: null,
      indexProgress: null,
      indexError: null,
      wikiSummary: null,
      wikiError: null,
      searchIndexSummary: null,
      searchResults: [],
      searchQuery: '',
      searchError: null,
      citations: [],
      chatMessages: [],
      chatError: null,
      lastFileChange: '',
      lastFileChangeAt: null,
      databaseSchema: null,
      databaseViewEnabled: false,
    }),
}));
