import { create } from 'zustand';

type ActivePanel = 'chat' | 'graph';
type Theme = 'dark' | 'light';

interface AppState {
  activePanel: ActivePanel;
  appVersion: string;
  theme: Theme;
  lastFileChange: string | null;
  setActivePanel: (panel: ActivePanel) => void;
  setAppVersion: (version: string) => void;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  setLastFileChange: (path: string) => void;
}

export const useAppStore = create<AppState>((set) => ({
  activePanel: 'chat',
  appVersion: 'loading',
  theme: 'dark',
  lastFileChange: null,
  setActivePanel: (panel) => set({ activePanel: panel }),
  setAppVersion: (version) => set({ appVersion: version }),
  setTheme: (theme) => set({ theme }),
  toggleTheme: () => set((state) => ({ theme: state.theme === 'dark' ? 'light' : 'dark' })),
  setLastFileChange: (path) => set({ lastFileChange: path }),
}));
