import { create } from 'zustand';

type ActivePanel = 'chat' | 'graph';

interface AppState {
  activePanel: ActivePanel;
  appVersion: string;
  setActivePanel: (panel: ActivePanel) => void;
  setAppVersion: (version: string) => void;
}

export const useAppStore = create<AppState>((set) => ({
  activePanel: 'chat',
  appVersion: 'loading',
  setActivePanel: (panel) => set({ activePanel: panel }),
  setAppVersion: (version) => set({ appVersion: version }),
}));
