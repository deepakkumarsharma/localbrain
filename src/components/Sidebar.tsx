import { Search } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';

const sections = ['Files', 'Search', 'Wiki'];

export function Sidebar() {
  const { activePanel, theme, toggleTheme } = useAppStore();

  return (
    <aside className="flex h-full flex-col border-r border-app-border bg-app-panel">
      <div className="border-b border-app-border px-4 py-4">
        <h1 className="text-lg font-semibold leading-tight">Localbrain</h1>
        <p className="mt-1 text-xs font-medium text-app-muted">Local code intelligence</p>
      </div>

      <div className="px-3 py-3">
        <div className="flex h-9 items-center gap-2 rounded-md border border-app-border bg-app-panelSoft px-3 text-app-muted">
          <Search className="h-4 w-4" aria-hidden="true" />
          <span className="text-sm">Search</span>
          <kbd className="ml-auto rounded border border-app-border px-1.5 py-0.5 font-mono text-[11px]">
            Cmd K
          </kbd>
        </div>
      </div>

      <nav className="space-y-1 px-2">
        {sections.map((section) => (
          <button
            key={section}
            className="flex h-8 w-full items-center rounded-md px-2 text-left text-sm font-medium text-app-muted hover:bg-app-panelSoft hover:text-app-text"
            type="button"
          >
            {section}
          </button>
        ))}
      </nav>

      <div className="mt-auto space-y-3 border-t border-app-border px-4 py-3 text-xs text-app-muted">
        <button
          className="flex h-8 w-full items-center justify-between rounded-md border border-app-border px-2 text-left font-medium text-app-text hover:bg-app-panelSoft"
          type="button"
          onClick={toggleTheme}
        >
          <span className="capitalize">{theme} theme</span>
          <kbd className="rounded border border-app-border px-1.5 py-0.5 font-mono text-[11px] text-app-muted">
            {navigator.platform.includes('Mac') ? 'Cmd' : 'Ctrl'} Shift T
          </kbd>
        </button>
        <p>Active: {activePanel}</p>
      </div>
    </aside>
  );
}
