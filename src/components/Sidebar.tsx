import { Search } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';

const sections = ['Files', 'Search', 'Wiki'];

export function Sidebar() {
  const { activePanel, theme, toggleTheme } = useAppStore();

  return (
    <aside className="flex h-full min-w-[260px] max-w-[380px] flex-col bg-app-panel min-[1440px]:min-w-[400px] min-[1440px]:max-w-[600px]">
      <div className="border-b border-app-border px-6 py-5">
        <h1 className="text-xl font-semibold leading-tight">Localbrain</h1>
        <p className="mt-1.5 text-sm font-medium text-app-muted">Local code intelligence</p>
      </div>

      <div className="px-5 py-5">
        <div className="flex h-11 items-center gap-3 rounded-md border border-app-border bg-app-panelSoft px-4 text-app-muted">
          <Search className="h-5 w-5" aria-hidden="true" />
          <span className="text-[15px]">Search</span>
          <kbd className="ml-auto rounded border border-app-border px-2 py-1 font-mono text-xs">
            Cmd K
          </kbd>
        </div>
      </div>

      <nav className="space-y-2 px-4">
        {sections.map((section) => (
          <button
            key={section}
            className="flex h-10 w-full items-center rounded-md px-3 text-left text-[15px] font-medium text-app-muted hover:bg-app-panelSoft hover:text-app-text"
            type="button"
          >
            {section}
          </button>
        ))}
      </nav>

      <div className="mt-auto space-y-4 border-t border-app-border px-6 py-5 text-sm text-app-muted">
        <button
          className="flex min-h-10 w-full items-center justify-between gap-3 rounded-md border border-app-border px-3 text-left font-medium text-app-text hover:bg-app-panelSoft"
          type="button"
          onClick={toggleTheme}
        >
          <span className="capitalize">{theme} theme</span>
          <kbd className="shrink-0 rounded border border-app-border px-2 py-1 font-mono text-xs text-app-muted">
            {navigator.platform.includes('Mac') ? 'Cmd' : 'Ctrl'} Shift T
          </kbd>
        </button>
        <p>Active: {activePanel}</p>
      </div>
    </aside>
  );
}
