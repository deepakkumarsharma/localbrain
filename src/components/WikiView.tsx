import { BookText, FileCode2, ShieldCheck, Sparkles } from 'lucide-react';
import { marked } from 'marked';
import { useEffect, useState } from 'react';
import { getWikiContent } from '../lib/wiki';
import { useAppStore } from '../store/useAppStore';

export function WikiView() {
  const { wikiSummary, activeSourcePath, setActivePanel, setSelectedGraphNode, graphView } =
    useAppStore();
  const [content, setContent] = useState<string | null>(null);
  const [html, setHtml] = useState<string>('');

  useEffect(() => {
    void getWikiContent(activeSourcePath).then((res) => {
      setContent(res);
    });
  }, [activeSourcePath, wikiSummary]);

  useEffect(() => {
    if (!content) {
      setHtml('');
      return;
    }

    const rawHtml = marked.parse(content, { async: false }) as string;
    const processedHtml = rawHtml.replace(
      /\[\[([^\]]+)\]\]/g,
      '<button type="button" class="wikilink" data-node="$1">$1</button>',
    );

    setHtml(processedHtml);
  }, [content]);

  const handleWikiClick = (event: React.MouseEvent) => {
    const target = event.target as HTMLElement;
    if (target.classList.contains('wikilink')) {
      const nodeId = target.getAttribute('data-node');
      if (nodeId && graphView) {
        const node = graphView.nodes.find((n) => n.id === nodeId || n.label === nodeId);
        if (node) {
          setSelectedGraphNode(node);
          setActivePanel('graph');
        }
      }
    }
  };

  const sourceFile = activeSourcePath || 'No file selected';
  const isReady = Boolean(content);

  return (
    <div className="absolute inset-0 bg-app-background">
      <div className="app-scrollbar h-full overflow-auto">
        <div className="mx-auto w-full max-w-[1040px] px-8 py-8">
          <div className="mb-6 rounded-2xl border border-app-border bg-app-panel/90 p-5 shadow-[0_10px_30px_-18px_rgba(0,0,0,0.35)]">
            <div className="flex items-start justify-between gap-4">
              <div className="min-w-0">
                <div className="flex items-center gap-2 text-app-muted">
                  <BookText className="h-4 w-4" aria-hidden="true" />
                  <span className="text-[11px] font-black uppercase tracking-widest">
                    Wiki View
                  </span>
                </div>
                <h1 className="mt-2 truncate text-[24px] font-black tracking-tight text-app-text">
                  {wikiFileName(activeSourcePath)}
                </h1>
                <p className="mt-1 truncate text-[13px] text-app-muted" title={sourceFile}>
                  Source file: <span className="font-mono text-app-text">{sourceFile}</span>
                </p>
              </div>
              <span className="inline-flex shrink-0 items-center gap-1.5 rounded-full border border-emerald-500/25 bg-emerald-500/10 px-3 py-1.5 text-[11px] font-bold text-emerald-500">
                <ShieldCheck className="h-3.5 w-3.5" aria-hidden="true" />
                Source-backed
              </span>
            </div>
            <div className="mt-4 grid grid-cols-1 gap-2 sm:grid-cols-3">
              <MetaPill
                icon={<FileCode2 className="h-3.5 w-3.5" aria-hidden="true" />}
                label="Wiki Path"
                value={`docs/wiki/${wikiFileName(activeSourcePath)}`}
                mono
              />
              <MetaPill
                icon={<Sparkles className="h-3.5 w-3.5" aria-hidden="true" />}
                label="Pages Generated"
                value={wikiSummary ? String(wikiSummary.pagesWritten) : 'N/A'}
              />
              <MetaPill
                icon={<BookText className="h-3.5 w-3.5" aria-hidden="true" />}
                label="Status"
                value={isReady ? 'Loaded' : 'Not generated'}
              />
            </div>
          </div>

          <div className="rounded-2xl border border-app-border bg-app-panel p-0 shadow-[0_20px_35px_-26px_rgba(0,0,0,0.45)]">
            {content ? (
              <article
                className="wiki-markdown p-7 sm:p-10"
                dangerouslySetInnerHTML={{ __html: html }}
                onClick={handleWikiClick}
              />
            ) : (
              <div className="flex min-h-[380px] flex-col items-center justify-center px-6 py-14 text-center">
                <div className="mb-4 rounded-full border border-app-border bg-app-background p-3">
                  <BookText className="h-6 w-6 text-app-muted" aria-hidden="true" />
                </div>
                <h2 className="text-[20px] font-black tracking-tight text-app-text">
                  No wiki page available yet
                </h2>
                <p className="mt-2 max-w-[560px] text-[14px] leading-7 text-app-muted">
                  Generate wiki from the top action bar to create source-backed documentation for
                  this file.
                </p>
                <button
                  className="mt-6 rounded-xl border border-app-border bg-app-background px-4 py-2.5 text-[13px] font-bold text-app-text hover:border-app-accent/40 hover:text-app-accent transition-colors"
                  onClick={() => setActivePanel('graph')}
                >
                  Switch to Graph View
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
      <style>{`
        .wiki-markdown {
          color: rgb(var(--color-app-text));
          font-size: 15px;
          line-height: 1.8;
        }
        .wiki-markdown h1,
        .wiki-markdown h2,
        .wiki-markdown h3 {
          color: rgb(var(--color-app-text));
          font-weight: 800;
          letter-spacing: -0.02em;
          margin-top: 1.8rem;
          margin-bottom: 0.8rem;
          line-height: 1.25;
        }
        .wiki-markdown h1 { font-size: 1.8rem; margin-top: 0.2rem; }
        .wiki-markdown h2 { font-size: 1.35rem; }
        .wiki-markdown h3 { font-size: 1.12rem; }
        .wiki-markdown p,
        .wiki-markdown li {
          color: rgb(var(--color-app-muted));
        }
        .wiki-markdown ul,
        .wiki-markdown ol {
          margin: 0.7rem 0 1rem 1.2rem;
        }
        .wiki-markdown li {
          margin: 0.22rem 0;
          padding-left: 0.2rem;
        }
        .wiki-markdown a {
          color: rgb(var(--color-app-accent));
          text-underline-offset: 3px;
        }
        .wiki-markdown hr {
          border: 0;
          border-top: 1px solid rgb(var(--color-app-border));
          margin: 1.4rem 0;
        }
        .wiki-markdown blockquote {
          margin: 1rem 0;
          padding: 0.7rem 1rem;
          border-left: 3px solid rgb(var(--color-app-accent));
          background: rgb(var(--color-app-panel-soft));
          border-radius: 0.45rem;
          color: rgb(var(--color-app-text));
        }
        .wiki-markdown code {
          padding: 0.12rem 0.45rem;
          border-radius: 0.4rem;
          border: 1px solid rgb(var(--color-app-border));
          background: rgb(var(--color-app-panel-soft));
          font-size: 12px;
          color: rgb(var(--color-app-text));
          font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
        }
        .wiki-markdown pre {
          margin: 1rem 0;
          padding: 0.9rem 1rem;
          border-radius: 0.8rem;
          border: 1px solid rgb(var(--color-app-border));
          background: rgb(var(--color-app-background));
          overflow: auto;
        }
        .wiki-markdown pre code {
          display: block;
          padding: 0;
          border: none;
          background: transparent;
          line-height: 1.65;
        }
        .wiki-markdown table {
          width: 100%;
          border-collapse: collapse;
          margin: 1rem 0;
          font-size: 13px;
        }
        .wiki-markdown th,
        .wiki-markdown td {
          border: 1px solid rgb(var(--color-app-border));
          padding: 0.55rem 0.65rem;
          text-align: left;
        }
        .wiki-markdown th {
          background: rgb(var(--color-app-panel-soft));
          color: rgb(var(--color-app-text));
          font-weight: 700;
        }
        .wikilink {
          display: inline-flex;
          align-items: center;
          gap: 0.2rem;
          color: rgb(var(--color-app-accent));
          border: 1px solid rgba(var(--color-app-accent), 0.35);
          border-radius: 999px;
          padding: 0.05rem 0.5rem;
          background: rgba(var(--color-app-accent), 0.08);
          cursor: pointer;
          transition: background-color 0.2s, border-color 0.2s;
          font-size: 12px;
          font-weight: 700;
        }
        .wikilink:hover {
          background-color: rgba(var(--color-app-accent), 0.16);
          border-color: rgba(var(--color-app-accent), 0.55);
        }
      `}</style>
    </div>
  );
}

function MetaPill({
  icon,
  label,
  value,
  mono,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex items-start gap-2 rounded-xl border border-app-border bg-app-background/70 px-3 py-2.5">
      <span className="mt-0.5 text-app-muted">{icon}</span>
      <span className="min-w-0">
        <span className="block text-[10px] font-black uppercase tracking-widest text-app-muted">
          {label}
        </span>
        <span
          className={`block truncate text-[12px] font-semibold text-app-text ${mono ? 'font-mono' : ''}`}
          title={value}
        >
          {value}
        </span>
      </span>
    </div>
  );
}

function wikiFileName(path: string) {
  return `${path.replace(/[\\/]/g, '_')}.md`;
}
