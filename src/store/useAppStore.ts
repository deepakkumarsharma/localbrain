import { create } from 'zustand';

type ActivePanel = 'chat' | 'graph';
type Theme = 'dark' | 'light';

interface AppState {
  activePanel: ActivePanel;
  appVersion: string;
  theme: Theme;
  setActivePanel: (panel: ActivePanel) => void;
  setAppVersion: (version: string) => void;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  activePanel: 'chat',
  appVersion: 'loading',
  theme: 'dark',
  setActivePanel: (panel) => set({ activePanel: panel }),
  setAppVersion: (version) => set({ appVersion: version }),
  setTheme: (theme) => set({ theme }),
  toggleTheme: () => set((state) => ({ theme: state.theme === 'dark' ? 'light' : 'dark' })),
}));
