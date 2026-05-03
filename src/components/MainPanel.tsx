import type { FormEvent } from 'react';
import { Send } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';

export function MainPanel() {
  const { activePanel, setActivePanel } = useAppStore();

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
  }

  return (
    <main className="flex h-full min-w-0 flex-col bg-app-background">
      <header className="flex h-14 items-center justify-between border-b border-app-border px-4">
        <div>
          <h2 className="text-lg font-semibold">Query</h2>
          <p className="text-xs font-medium text-app-muted">Ask about a local repository</p>
        </div>
        <div className="flex rounded-md border border-app-border bg-app-panel p-1">
          {(['chat', 'graph'] as const).map((panel) => (
            <button
              key={panel}
              className="h-7 rounded px-3 text-xs font-medium capitalize text-app-muted data-[active=true]:bg-app-accentSoft data-[active=true]:text-app-text"
              data-active={activePanel === panel}
              type="button"
              onClick={() => setActivePanel(panel)}
            >
              {panel}
            </button>
          ))}
        </div>
      </header>

      <section className="flex flex-1 items-center justify-center px-8">
        <div className="max-w-md text-center">
          <h3 className="text-lg font-semibold">Index a repository to get started</h3>
          <p className="mt-2 text-sm leading-6 text-app-muted">
            Drop a folder here or choose a recent repository once indexing is available.
          </p>
        </div>
      </section>

      <form className="border-t border-app-border bg-app-panel px-4 py-3" onSubmit={handleSubmit}>
        <div className="flex items-center gap-3 rounded-md border border-app-border bg-app-panelSoft px-3 py-2">
          <input
            className="min-w-0 flex-1 bg-transparent text-sm leading-6 text-app-text outline-none placeholder:text-app-muted"
            placeholder="Ask Localbrain..."
            type="text"
          />
          <button
            className="inline-flex h-8 w-8 items-center justify-center rounded-md bg-app-accentSoft text-app-accent hover:bg-app-accent hover:text-app-background"
            type="button"
            aria-label="Submit query"
          >
            <Send className="h-4 w-4" aria-hidden="true" />
          </button>
        </div>
      </form>
    </main>
  );
}
