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
  const [outline, setOutline] = useState<Array<{ title: string; level: number; id: string }>>([]);

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
    const outlineItems: Array<{ title: string; level: number; id: string }> = [];
    const withAnchors = rawHtml.replace(
      /<(h[1-3])>(.*?)<\/h[1-3]>/g,
      (_full: string, tag: string, inner: string) => {
        const plain = stripTags(inner).trim();
        const slug = headingSlug(plain);
        const level = Number(tag.slice(1));
        outlineItems.push({ title: plain, level, id: slug });
        return `<${tag} id="${slug}">${inner}</${tag}>`;
      },
    );
    const processedHtml = withAnchors.replace(/\[\[([^\]]+)\]\]/g, (_, capture: string) => {
      const escaped = escapeHtml(capture.trim());
      return `<button type="button" class="wikilink" data-node="${escaped}">${escaped}</button>`;
    });

    setOutline(outlineItems);
    setHtml(sanitizeHtml(processedHtml));
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
  const wikiStats = deriveWikiStats(content);
  const insights = deriveDeveloperInsights(content);

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

          <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_300px]">
            <div className="rounded-2xl border border-app-border bg-app-panel p-0 shadow-[0_20px_35px_-26px_rgba(0,0,0,0.45)] overflow-hidden">
              {content ? (
                <>
                  <div className="border-b border-app-border bg-gradient-to-r from-blue-500/10 via-violet-500/10 to-emerald-500/10 px-7 py-3 text-[11px] font-black uppercase tracking-widest text-app-muted">
                    Developer Overview
                  </div>
                  <article
                    className="wiki-markdown p-7 sm:p-10"
                    dangerouslySetInnerHTML={{ __html: html }}
                    onClick={handleWikiClick}
                  />
                </>
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
            {content ? (
              <aside className="rounded-2xl border border-app-border bg-app-panel p-4 h-fit">
                <h3 className="text-[11px] font-black uppercase tracking-widest text-app-muted">
                  Document Intelligence
                </h3>
                <div className="mt-3 grid grid-cols-3 gap-2 text-center">
                  <MiniStat label="Sections" value={String(wikiStats.sections)} />
                  <MiniStat label="Code" value={String(wikiStats.codeBlocks)} />
                  <MiniStat label="Links" value={String(wikiStats.links)} />
                </div>
                <div className="mt-2 grid grid-cols-2 gap-2 text-center">
                  <MiniStat label="Words" value={String(wikiStats.words)} />
                  <MiniStat label="Read (min)" value={String(wikiStats.readMinutes)} />
                </div>
                <h4 className="mt-4 text-[11px] font-black uppercase tracking-widest text-app-muted">
                  Developer Signals
                </h4>
                <div className="mt-2 space-y-1.5">
                  {insights.map((insight) => (
                    <div
                      key={insight.label}
                      className={`rounded-lg border px-2.5 py-1.5 text-[11px] font-bold ${
                        insight.ok
                          ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-400'
                          : 'border-amber-500/30 bg-amber-500/10 text-amber-400'
                      }`}
                    >
                      {insight.label}
                    </div>
                  ))}
                </div>
                <h4 className="mt-4 text-[11px] font-black uppercase tracking-widest text-app-muted">
                  Outline
                </h4>
                <div className="mt-2 max-h-[360px] overflow-auto app-scrollbar pr-1 space-y-1">
                  {outline.length > 0 ? (
                    outline.map((item) => (
                      <a
                        key={item.id}
                        href={`#${item.id}`}
                        className="block rounded-md px-2 py-1 text-[12px] text-app-muted hover:bg-app-background hover:text-app-text"
                        style={{ paddingLeft: `${Math.max(8, item.level * 10)}px` }}
                      >
                        {item.title}
                      </a>
                    ))
                  ) : (
                    <div className="text-[12px] text-app-muted">No sections detected.</div>
                  )}
                </div>
              </aside>
            ) : null}
          </div>
        </div>

        {content ? (
          <article
            className="wiki-content prose prose-invert max-w-none"
            dangerouslySetInnerHTML={{ __html: html }}
            onClick={handleWikiClick}
          />
        ) : (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <p className="text-app-muted mb-4">No wiki page found for this file.</p>
            <button
              className="px-4 py-2 rounded-lg bg-app-accent hover:bg-app-accentSoft text-white text-sm font-medium transition-colors"
              onClick={() => setActivePanel('graph')}
            >
              Generate Wiki via Command Palette (⌘K)
            </button>
          </div>
        )}
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
          color: color-mix(in srgb, rgb(var(--color-app-text)) 88%, rgb(var(--color-app-muted)) 12%);
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
          padding: 0.9rem 1rem;
          border-left: 3px solid rgb(var(--color-app-accent));
          background: linear-gradient(90deg, rgba(var(--color-app-accent),0.12), rgb(var(--color-app-panel-soft)));
          border-radius: 0.6rem;
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
          overflow: hidden;
          border-radius: 0.6rem;
        }
        .wiki-markdown th,
        .wiki-markdown td {
          border: 1px solid rgb(var(--color-app-border));
          padding: 0.55rem 0.65rem;
          text-align: left;
        }
        .wiki-markdown th {
          background: linear-gradient(90deg, rgba(var(--color-app-accent),0.12), rgb(var(--color-app-panel-soft)));
          color: rgb(var(--color-app-text));
          font-weight: 700;
        }
        .wikilink {
          color: rgb(var(--color-graph-component));
          border-bottom: 1px dotted rgba(var(--color-graph-component), 0.4);
          cursor: pointer;
          transition: background-color 0.2s;
        }
        .wikilink:hover {
          background-color: rgba(var(--color-graph-component), 0.1);
        }
      `}</style>
    </div>
  );
}

function stripTags(value: string) {
  return value.replace(/<[^>]+>/g, '');
}

function headingSlug(value: string) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-');
}

function deriveWikiStats(content: string | null) {
  if (!content) return { sections: 0, codeBlocks: 0, links: 0, words: 0, readMinutes: 0 };
  const sections = (content.match(/^#{1,3}\s+/gm) || []).length;
  const codeBlocks = (content.match(/```/g) || []).length / 2;
  const links = (content.match(/\[[^\]]+\]\([^)]+\)/g) || []).length;
  const words = content.split(/\s+/).filter(Boolean).length;
  const readMinutes = Math.max(1, Math.round(words / 220));
  return {
    sections,
    codeBlocks: Math.max(0, Math.floor(codeBlocks)),
    links,
    words,
    readMinutes,
  };
}

function deriveDeveloperInsights(content: string | null) {
  const value = (content || '').toLowerCase();
  return [
    { label: 'Has setup/install details', ok: /install|setup|run|start/.test(value) },
    { label: 'Has architecture notes', ok: /architecture|flow|design|module/.test(value) },
    { label: 'Has API/endpoint references', ok: /api|endpoint|route|http/.test(value) },
    {
      label: 'Has debugging/troubleshooting hints',
      ok: /error|debug|troubleshoot|issue/.test(value),
    },
  ];
}

function MiniStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-app-border bg-app-background px-2 py-2">
      <div className="text-[14px] font-black text-app-text">{value}</div>
      <div className="text-[10px] uppercase tracking-widest text-app-muted">{label}</div>
    </div>
  );
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function sanitizeHtml(value: string): string {
  const parser = new DOMParser();
  const doc = parser.parseFromString(value, 'text/html');

  doc.querySelectorAll('script, style, iframe, object, embed, link, meta').forEach((node) => {
    node.remove();
  });

  doc.querySelectorAll<HTMLElement>('*').forEach((element) => {
    const attrs = Array.from(element.attributes);
    for (const attr of attrs) {
      const name = attr.name.toLowerCase();
      const val = attr.value.trim().toLowerCase();
      if (name.startsWith('on') || val.startsWith('javascript:')) {
        element.removeAttribute(attr.name);
      }
    }
  });

  return doc.body.innerHTML;
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
