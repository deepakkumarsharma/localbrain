import { Code2, Database, ExternalLink, FileCheck2, FolderSync } from 'lucide-react';
import { getGraphSymbols, indexFileToGraph } from '../lib/graph';
import { indexFile, indexPath } from '../lib/indexer';
import { recordFileMetadata } from '../lib/metadata';
import { parseSourceFile } from '../lib/parser';
import { useAppStore } from '../store/useAppStore';

const DEMO_SOURCE_PATH = import.meta.env.DEV ? 'src/App.tsx' : null;
const ACTION_BUTTON_CLASS =
  'inline-flex h-9 items-center gap-2 rounded-md border border-app-border px-3 text-[15px] font-medium text-app-text hover:bg-app-panelSoft disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-transparent';

export function RightPanel() {
  const {
    appVersion,
    lastFileChange,
    lastFileChangeAt,
    parsedFile,
    parserError,
    graphSummary,
    graphSymbols,
    graphError,
    metadata,
    metadataError,
    indexFileSummary,
    indexPathSummary,
    indexRun,
    indexError,
    setParsedFile,
    setParserError,
    setGraphResult,
    setGraphError,
    setMetadataResult,
    setMetadataError,
    setIndexFileResult,
    setIndexPathResult,
    setIndexError,
  } = useAppStore();

  async function handleParseApp() {
    if (!DEMO_SOURCE_PATH) {
      return;
    }

    try {
      const parsed = await parseSourceFile(DEMO_SOURCE_PATH);
      setParsedFile(parsed);
    } catch (error) {
      setParserError(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleIndexApp() {
    if (!DEMO_SOURCE_PATH) {
      return;
    }

    try {
      const summary = await indexFileToGraph(DEMO_SOURCE_PATH);
      const symbols = await getGraphSymbols(DEMO_SOURCE_PATH);
      setGraphResult(summary, symbols);
    } catch (error) {
      setGraphError(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleRecordMetadata() {
    if (!DEMO_SOURCE_PATH) {
      return;
    }

    try {
      const metadata = await recordFileMetadata(DEMO_SOURCE_PATH);
      setMetadataResult(metadata);
    } catch (error) {
      setMetadataError(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleIncrementalIndexApp() {
    if (!DEMO_SOURCE_PATH) {
      return;
    }

    try {
      const summary = await indexFile(DEMO_SOURCE_PATH);
      setIndexFileResult(summary);
      const symbols = await getGraphSymbols(DEMO_SOURCE_PATH);
      if (summary.graph) {
        setGraphResult(summary.graph, symbols);
      }
    } catch (error) {
      setIndexError(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleIncrementalIndexProject() {
    if (!DEMO_SOURCE_PATH) {
      return;
    }

    try {
      const summary = await indexPath('.');
      setIndexPathResult(summary);
    } catch (error) {
      setIndexError(error instanceof Error ? error.message : String(error));
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
          {lastFileChange ? (
            <div className="mt-3 space-y-1">
              <p className="break-all text-[15px] leading-7 text-app-muted">{lastFileChange}</p>
              {lastFileChangeAt ? (
                <p className="text-[13px] text-app-muted/60">
                  Last updated: {new Date(lastFileChangeAt).toLocaleTimeString()}
                </p>
              ) : null}
            </div>
          ) : (
            <p className="mt-3 text-[15px] leading-7 text-app-muted">
              No file changes detected yet.
            </p>
          )}
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
          <h3 className="text-[13px] font-semibold uppercase text-app-muted">Graph</h3>
          {graphSummary ? (
            <div className="mt-3 space-y-3">
              <p className="break-all text-[15px] leading-7 text-app-muted">
                {graphSummary.filePath} · {graphSummary.symbolCount} symbols ·{' '}
                {graphSummary.containsCount} contains edges
              </p>
              <div className="max-h-48 space-y-2 overflow-auto pr-1">
                {graphSymbols.map((symbol) => (
                  <div
                    key={`graph-${symbol.kind}-${symbol.name}-${symbol.range.startLine}`}
                    className="rounded-md border border-app-border px-3 py-2 text-sm"
                  >
                    <span className="font-medium text-app-text">{symbol.name}</span>
                    <span className="ml-2 text-app-muted">
                      {symbol.kind} · L{symbol.range.startLine}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <p className="mt-3 text-[15px] leading-7 text-app-muted">No graph data loaded yet.</p>
          )}
          {graphError ? (
            <p className="mt-3 break-all text-[15px] leading-7 text-red-400">{graphError}</p>
          ) : null}
        </div>

        <div>
          <h3 className="text-[13px] font-semibold uppercase text-app-muted">Metadata</h3>
          {metadata ? (
            <div className="mt-3 rounded-md border border-app-border px-3 py-2 text-sm">
              <p className="break-all font-medium text-app-text">{metadata.path}</p>
              <p className="mt-1 text-app-muted">
                {metadata.status} · {metadata.sizeBytes} bytes
              </p>
              <p className="mt-1 break-all text-app-muted">
                hash {metadata.contentHash.slice(0, 12)}
              </p>
              <p className="mt-1 text-app-muted">indexed {metadata.lastIndexedAt ?? 'not yet'}</p>
            </div>
          ) : (
            <p className="mt-3 text-[15px] leading-7 text-app-muted">No metadata recorded yet.</p>
          )}
          {metadataError ? (
            <p className="mt-3 break-all text-[15px] leading-7 text-red-400">{metadataError}</p>
          ) : null}
        </div>

        <div>
          <h3 className="text-[13px] font-semibold uppercase text-app-muted">Indexer</h3>
          {indexFileSummary ? (
            <p className="mt-3 break-all text-[15px] leading-7 text-app-muted">
              {indexFileSummary.path} · {indexFileSummary.status}
              {indexFileSummary.skipped ? ' · skipped' : ''}
            </p>
          ) : (
            <p className="mt-3 text-[15px] leading-7 text-app-muted">No indexed file yet.</p>
          )}
          {indexPathSummary ? (
            <p className="mt-2 text-[15px] leading-7 text-app-muted">
              Path run: {indexPathSummary.filesChanged}/{indexPathSummary.filesSeen} changed,{' '}
              {indexPathSummary.filesSkipped} skipped
            </p>
          ) : null}
          {indexRun ? (
            <p className="mt-2 text-[15px] leading-7 text-app-muted">
              Run {indexRun.id}: {indexRun.status}
            </p>
          ) : null}
          {indexError ? (
            <p className="mt-3 whitespace-pre-wrap break-all text-[15px] leading-7 text-red-400">
              {indexError}
            </p>
          ) : null}
        </div>

        <div>
          <h3 className="text-[13px] font-semibold uppercase text-app-muted">Actions</h3>
          <div className="mt-3 flex flex-col gap-2">
            <button
              className={ACTION_BUTTON_CLASS}
              disabled={!DEMO_SOURCE_PATH}
              type="button"
              onClick={handleParseApp}
            >
              <Code2 className="h-4 w-4" aria-hidden="true" />
              Parse App.tsx
            </button>
            <button
              className={ACTION_BUTTON_CLASS}
              disabled={!DEMO_SOURCE_PATH}
              type="button"
              onClick={handleIndexApp}
            >
              <Database className="h-4 w-4" aria-hidden="true" />
              Index App.tsx
            </button>
            <button
              className={ACTION_BUTTON_CLASS}
              disabled={!DEMO_SOURCE_PATH}
              type="button"
              onClick={handleRecordMetadata}
            >
              <FileCheck2 className="h-4 w-4" aria-hidden="true" />
              Record App.tsx Metadata
            </button>
            <button
              className={ACTION_BUTTON_CLASS}
              disabled={!DEMO_SOURCE_PATH}
              type="button"
              onClick={handleIncrementalIndexApp}
            >
              <Database className="h-4 w-4" aria-hidden="true" />
              Incremental Index App.tsx
            </button>
            <button
              className={ACTION_BUTTON_CLASS}
              disabled={!DEMO_SOURCE_PATH}
              type="button"
              onClick={handleIncrementalIndexProject}
            >
              <FolderSync className="h-4 w-4" aria-hidden="true" />
              Incremental Index Project
            </button>
            <button className={ACTION_BUTTON_CLASS} type="button">
              <ExternalLink className="h-4 w-4" aria-hidden="true" />
              Open in editor
            </button>
          </div>
        </div>
      </section>
    </aside>
  );
}
