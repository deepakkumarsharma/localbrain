import { Code2, ExternalLink } from 'lucide-react';
import { parseSourceFile } from '../lib/parser';
import { useAppStore } from '../store/useAppStore';

export function RightPanel() {
  const { appVersion, lastFileChange, parsedFile, parserError, setParsedFile, setParserError } =
    useAppStore();

  async function handleParseApp() {
    try {
      const parsed = await parseSourceFile('src/App.tsx');
      setParsedFile(parsed);
    } catch (error) {
      setParserError(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <aside className="flex h-full min-w-[260px] max-w-[380px] flex-col bg-app-panel min-[1440px]:min-w-[400px] min-[1440px]:max-w-[600px]">
      <header className="border-b border-app-border px-6 py-5">
        <h2 className="text-xl font-semibold leading-tight">Details</h2>
        <p className="mt-1.5 text-sm font-medium text-app-muted">Version {appVersion}</p>
      </header>

      <section className="space-y-6 overflow-auto px-6 py-6">
        <div>
          <h3 className="text-[13px] font-semibold uppercase text-app-muted">Citations</h3>
          <p className="mt-3 text-[15px] leading-7 text-app-muted">No citations yet.</p>
        </div>

        <div>
          <h3 className="text-[13px] font-semibold uppercase text-app-muted">Watcher</h3>
          <p className="mt-3 break-all text-[15px] leading-7 text-app-muted">
            {lastFileChange ?? 'No file changes detected yet.'}
          </p>
        </div>

        <div>
          <h3 className="text-[13px] font-semibold uppercase text-app-muted">Parser</h3>
          {parsedFile ? (
            <div className="mt-3 space-y-3">
              <p className="break-all text-[15px] leading-7 text-app-muted">
                {parsedFile.path} · {parsedFile.symbols.length} symbols
              </p>
              <div className="max-h-64 space-y-2 overflow-auto pr-1">
                {parsedFile.symbols.map((symbol) => (
                  <div
                    key={`${symbol.kind}-${symbol.name}-${symbol.range.startLine}`}
                    className="rounded-md border border-app-border px-3 py-2 text-sm"
                  >
                    <span className="font-medium text-app-text">{symbol.name}</span>
                    <span className="ml-2 text-app-muted">
                      {symbol.kind} · L{symbol.range.startLine}
                    </span>
                    {symbol.source ? (
                      <span className="block truncate text-app-muted">from {symbol.source}</span>
                    ) : null}
                    {symbol.parent ? (
                      <span className="block truncate text-app-muted">parent {symbol.parent}</span>
                    ) : null}
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <p className="mt-3 text-[15px] leading-7 text-app-muted">No parser output yet.</p>
          )}
          {parserError ? (
            <p className="mt-3 break-all text-[15px] leading-7 text-red-400">{parserError}</p>
          ) : null}
        </div>

        <div>
          <h3 className="text-[13px] font-semibold uppercase text-app-muted">Actions</h3>
          <div className="mt-3 flex flex-col gap-2">
            <button
              className="inline-flex h-9 items-center gap-2 rounded-md border border-app-border px-3 text-[15px] font-medium text-app-text hover:bg-app-panelSoft"
              type="button"
              onClick={handleParseApp}
            >
              <Code2 className="h-4 w-4" aria-hidden="true" />
              Parse App.tsx
            </button>
            <button
              className="inline-flex h-9 items-center gap-2 rounded-md border border-app-border px-3 text-[15px] font-medium text-app-text hover:bg-app-panelSoft"
              type="button"
            >
              <ExternalLink className="h-4 w-4" aria-hidden="true" />
              Open in editor
            </button>
          </div>
        </div>
      </section>
    </aside>
  );
}
