import type { FormEvent } from 'react';
import { useState } from 'react';
import { Send } from 'lucide-react';
import { hybridSearch } from '../lib/search';
import { useAppStore } from '../store/useAppStore';

export function MainPanel() {
  const {
    activePanel,
    searchResults,
    searchQuery,
    setActivePanel,
    setSearchError,
    setSearchResults,
  } = useAppStore();
  const [query, setQuery] = useState('');

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const trimmedQuery = query.trim();
    if (!trimmedQuery) {
      return;
    }

    try {
      const results = await hybridSearch(trimmedQuery, 8);
      setSearchResults(trimmedQuery, results);
    } catch (error) {
      setSearchError(error instanceof Error ? error.message : String(error));
    }
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

      <section className="flex-1 overflow-auto px-8 py-8">
        {searchResults.length > 0 ? (
          <div className="mx-auto max-w-3xl space-y-4">
            <div>
              <h3 className="text-lg font-semibold">Results for {searchQuery}</h3>
              <p className="mt-1 text-sm text-app-muted">{searchResults.length} matches</p>
            </div>
            <div className="space-y-3">
              {searchResults.map((result) => (
                <article
                  key={result.path}
                  className="rounded-md border border-app-border bg-app-panel px-4 py-3"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0">
                      <h4 className="truncate text-[15px] font-semibold">{result.title}</h4>
                      <p className="mt-1 break-all text-xs text-app-muted">{result.path}</p>
                    </div>
                    <span className="shrink-0 rounded border border-app-border px-2 py-1 text-xs uppercase text-app-muted">
                      {result.kind}
                    </span>
                  </div>
                  <p className="mt-3 text-sm leading-6 text-app-muted">{result.snippet}</p>
                  <p className="mt-2 text-xs text-app-muted/70">
                    score {result.score.toFixed(2)} · text {result.textScore.toFixed(2)} · vector{' '}
                    {result.vectorScore.toFixed(2)}
                  </p>
                </article>
              ))}
            </div>
          </div>
        ) : (
          <div className="flex h-full items-center justify-center">
            <div className="max-w-md text-center">
              <h3 className="text-lg font-semibold">Index a repository to get started</h3>
              <p className="mt-2 text-sm leading-6 text-app-muted">
                Build the wiki and search index, then ask about local code from here.
              </p>
            </div>
          </div>
        )}
      </section>

      <form className="border-t border-app-border bg-app-panel px-4 py-3" onSubmit={handleSubmit}>
        <div className="flex items-center gap-3 rounded-md border border-app-border bg-app-panelSoft px-3 py-2">
          <input
            className="min-w-0 flex-1 bg-transparent text-sm leading-6 text-app-text outline-none placeholder:text-app-muted"
            placeholder="Ask Localbrain..."
            type="text"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <button
            className="inline-flex h-8 w-8 items-center justify-center rounded-md bg-app-accentSoft text-app-accent hover:bg-app-accent hover:text-app-background"
            type="submit"
            aria-label="Submit query"
          >
            <Send className="h-4 w-4" aria-hidden="true" />
          </button>
        </div>
      </form>
    </main>
  );
}
