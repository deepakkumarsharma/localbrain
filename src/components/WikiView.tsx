import { ShieldCheck } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';

export function WikiView() {
  const { wikiSummary, activeSourcePath } = useAppStore();

  return (
    <div className="absolute inset-0 overflow-auto bg-app-background">
      <div className="mx-auto max-w-[860px] px-8 py-10">
        <div className="mb-6 flex items-center gap-2">
          <span className="rounded-md border border-app-border bg-app-panel px-2 py-1 font-mono text-[11px]">
            docs/wiki/{wikiFileName(activeSourcePath)}
          </span>
          <span className="inline-flex items-center gap-1 rounded-md border border-emerald-500/25 bg-emerald-500/10 px-2 py-1 text-[10px] font-medium text-emerald-400">
            <ShieldCheck className="h-3 w-3" aria-hidden="true" />
            Source-backed
          </span>
          <span className="text-[11px] text-app-muted">
            {wikiSummary
              ? `${wikiSummary.pagesWritten} pages generated`
              : 'Generate wiki to refresh'}
          </span>
        </div>

        <article className="max-w-none">
          <h1 className="mb-4 text-[28px] font-semibold">Localbrain Feature</h1>
          <p className="my-3 text-sm leading-7 text-app-muted">
            This page summarizes the selected source file and links it back to parser, graph,
            search, wiki, and local answer context.
          </p>

          <h2 className="mb-3 mt-8 text-lg font-semibold text-app-text">Source</h2>
          <p className="my-3 text-sm leading-7 text-app-muted">
            <span className="font-mono text-app-text">{activeSourcePath}</span> is parsed locally
            with Tree-sitter and indexed into metadata, graph, wiki, and search stores.
          </p>

          <h2 className="mb-3 mt-8 text-lg font-semibold text-app-text">Pipeline</h2>
          <div className="space-y-3 text-sm text-app-muted">
            <p>
              Parser extracts source symbols → graph stores relationships → wiki/search provide
              citations → chat answers with local evidence.
            </p>
            <p>
              The current answer path is retrieval-grounded. Real llama.cpp generation is not wired
              yet.
            </p>
          </div>
        </article>
      </div>
    </div>
  );
}

function wikiFileName(path: string) {
  return `${path.split('/').join('_')}.md`;
}
