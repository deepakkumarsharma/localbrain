import { X } from 'lucide-react';
import { friendlyErrorMessage } from '../lib/errors';
import { useAppStore } from '../store/useAppStore';

export function ErrorBanner() {
  const {
    chatError,
    graphError,
    indexError,
    parserError,
    searchError,
    wikiError,
    setChatError,
    setGraphError,
    setIndexError,
    setParserError,
    setSearchError,
    setWikiError,
  } = useAppStore();
  const error = chatError ?? searchError ?? graphError ?? indexError ?? wikiError ?? parserError;

  if (!error) {
    return null;
  }

  return (
    <div className="border-b border-app-border bg-app-error/10 px-4 py-2 text-sm text-app-text">
      <div className="flex items-start gap-3">
        <p className="min-w-0 flex-1 leading-6">{friendlyErrorMessage(error)}</p>
        <button
          className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded text-app-muted hover:bg-app-panelSoft hover:text-app-text"
          type="button"
          aria-label="Dismiss error"
          onClick={() => {
            setChatError(null);
            setGraphError(null);
            setIndexError(null);
            setParserError(null);
            setSearchError(null);
            setWikiError(null);
          }}
        >
          <X className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}
