import { BookText, ShieldCheck, CheckCircle2, AlertCircle, FileText, Clock } from 'lucide-react';
import { marked } from 'marked';
import { useEffect, useState } from 'react';
import { getWikiContent } from '../lib/wiki';
import { useAppStore } from '../store/useAppStore';

interface WikiViewProps {
  onGenerateWiki: () => void;
  isGeneratingWiki: boolean;
}

export function WikiView({ onGenerateWiki, isGeneratingWiki }: WikiViewProps) {
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
    const parser = new DOMParser();
    const doc = parser.parseFromString(rawHtml, 'text/html');
    const slugCounts = new Map<string, number>();
    doc.querySelectorAll('h1,h2,h3').forEach((el) => {
      const plain = stripTags(el.textContent || '').trim();
      const baseSlug = headingSlug(plain);
      const currentCount = slugCounts.get(baseSlug) ?? 0;
      const nextCount = currentCount + 1;
      slugCounts.set(baseSlug, nextCount);
      const uniqueSlug = nextCount > 1 ? `${baseSlug}-${nextCount}` : baseSlug;
      el.id = uniqueSlug;
      outlineItems.push({ title: plain, level: Number(el.tagName.slice(1)), id: uniqueSlug });
    });
    const withAnchors = doc.body.innerHTML;
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
  const wikiStats = deriveWikiStats(content);
  const insights = deriveDeveloperInsights(content);

  return (
    <div className="absolute inset-0 bg-app-background flex flex-col">
      <div className="app-scrollbar h-full overflow-y-auto">
        <div className="mx-auto flex w-full max-w-[1280px] flex-col lg:flex-row items-start gap-12 px-6 py-10 md:px-12">
          {content ? (
            <>
              {/* Main Document Area */}
              <main className="flex-1 min-w-0 max-w-[840px]">
                <header className="mb-10 border-b border-app-border pb-8">
                  <div className="flex items-center gap-3 text-[13px] text-app-muted mb-4 font-mono">
                    <BookText className="h-4 w-4 text-app-text" aria-hidden="true" />
                    {sourceFile}
                  </div>
                  <h1 className="text-[40px] font-bold tracking-tight text-app-text mb-6">
                    {wikiFileName(activeSourcePath).replace('.md', '')}
                  </h1>
                  <div className="flex flex-wrap items-center gap-4 text-[13px] text-app-muted font-medium">
                    <span className="inline-flex items-center gap-1.5 text-app-success bg-app-success/10 px-2 py-1 rounded">
                      <ShieldCheck className="h-4 w-4" aria-hidden="true" />
                      Source-backed
                    </span>
                    <span className="flex items-center gap-1.5">
                      <Clock className="h-4 w-4" /> {wikiStats.readMinutes} min read
                    </span>
                    <span className="flex items-center gap-1.5">
                      <FileText className="h-4 w-4" /> {wikiStats.words} words
                    </span>
                  </div>
                </header>

                <article
                  className="wiki-markdown"
                  dangerouslySetInnerHTML={{ __html: html }}
                  onClick={handleWikiClick}
                />
              </main>

              {/* Developer Sidebar */}
              <aside className="sticky top-10 hidden w-[280px] shrink-0 lg:flex flex-col gap-10">
                
                {/* Outline */}
                <div>
                  <h3 className="text-[13px] font-bold text-app-text mb-3 uppercase tracking-wider">
                    On this page
                  </h3>
                  <nav className="flex flex-col gap-1.5 border-l border-app-border">
                    {outline.length > 0 ? (
                      outline.map((item) => (
                        <a
                          key={item.id}
                          href={`#${item.id}`}
                          className="block text-[13.5px] text-app-muted hover:text-app-text transition-colors -ml-[1px] border-l-2 border-transparent hover:border-app-text py-0.5"
                          style={{ paddingLeft: `${Math.max(16, item.level * 12)}px` }}
                        >
                          {item.title}
                        </a>
                      ))
                    ) : (
                      <div className="text-[13px] text-app-muted pl-4">No sections found.</div>
                    )}
                  </nav>
                </div>

                {/* Developer Signals */}
                <div>
                  <h3 className="text-[13px] font-bold text-app-text mb-3 uppercase tracking-wider">
                    Developer Signals
                  </h3>
                  <ul className="flex flex-col gap-2.5">
                    {insights.map((insight) => (
                      <li key={insight.label} className="flex items-start gap-2.5">
                        {insight.ok ? (
                          <CheckCircle2 className="h-4 w-4 text-app-success shrink-0 mt-0.5" />
                        ) : (
                          <AlertCircle className="h-4 w-4 text-app-warning shrink-0 mt-0.5" />
                        )}
                        <span className={`text-[13.5px] leading-snug ${insight.ok ? 'text-app-text' : 'text-app-muted'}`}>
                          {insight.label}
                        </span>
                      </li>
                    ))}
                  </ul>
                </div>
              </aside>
            </>
          ) : (
            <div className="flex w-full min-h-[60vh] flex-col items-center justify-center text-center">
              <div className="mb-6 rounded-xl border border-app-border bg-app-panel p-5 text-app-muted shadow-sm">
                <FileText className="h-10 w-10" aria-hidden="true" />
              </div>
              <h2 className="text-[22px] font-bold tracking-tight text-app-text mb-2">
                No developer documentation
              </h2>
              <p className="max-w-[480px] text-[15px] text-app-muted mb-8 leading-relaxed">
                Wiki content is generated directly from parser symbols and structural data. Generate it to explore <code className="text-app-text font-mono bg-app-panel px-1.5 py-0.5 rounded border border-app-border text-[13px]">{sourceFile}</code>.
              </p>
              <div className="flex gap-4">
                <button
                  className="rounded-md bg-app-text px-4 py-2 text-[14px] font-bold text-app-background hover:opacity-90 disabled:opacity-50 transition-opacity"
                  onClick={onGenerateWiki}
                  disabled={isGeneratingWiki}
                >
                  {isGeneratingWiki ? 'Generating...' : 'Generate Wiki'}
                </button>
                <button
                  className="rounded-md border border-app-border bg-app-panel px-4 py-2 text-[14px] font-bold text-app-text hover:bg-app-panelSoft transition-colors"
                  onClick={() => setActivePanel('graph')}
                >
                  Return to Graph
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      <style>{`
        /* Professional Developer Docs Markdown Styling (GitHub / Stripe inspired) */
        .wiki-markdown {
          color: rgb(var(--color-app-text));
          font-size: 15px;
          line-height: 1.6;
          word-wrap: break-word;
        }
        .wiki-markdown > *:first-child {
          margin-top: 0 !important;
        }
        .wiki-markdown h1,
        .wiki-markdown h2,
        .wiki-markdown h3,
        .wiki-markdown h4 {
          color: rgb(var(--color-app-text));
          font-weight: 600;
          margin-top: 24px;
          margin-bottom: 16px;
          line-height: 1.25;
        }
        .wiki-markdown h1 {
          font-size: 2em;
          padding-bottom: 0.3em;
          border-bottom: 1px solid rgb(var(--color-app-border));
        }
        .wiki-markdown h2 {
          font-size: 1.5em;
          padding-bottom: 0.3em;
          border-bottom: 1px solid rgb(var(--color-app-border));
        }
        .wiki-markdown h3 {
          font-size: 1.25em;
        }
        .wiki-markdown h4 {
          font-size: 1em;
        }
        .wiki-markdown p,
        .wiki-markdown blockquote,
        .wiki-markdown ul,
        .wiki-markdown ol,
        .wiki-markdown dl,
        .wiki-markdown table,
        .wiki-markdown pre {
          margin-top: 0;
          margin-bottom: 16px;
        }
        .wiki-markdown a {
          color: rgb(var(--color-app-accent));
          text-decoration: none;
        }
        .wiki-markdown a:hover {
          text-decoration: underline;
        }
        .wiki-markdown ul,
        .wiki-markdown ol {
          padding-left: 2em;
        }
        .wiki-markdown li {
          margin-top: 0.25em;
        }
        .wiki-markdown li > p {
          margin-top: 16px;
        }
        .wiki-markdown code {
          padding: 0.2em 0.4em;
          margin: 0;
          font-size: 85%;
          background-color: rgba(var(--color-app-text), 0.08);
          border-radius: 6px;
          font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace;
        }
        .wiki-markdown pre {
          padding: 16px;
          overflow: auto;
          font-size: 85%;
          line-height: 1.45;
          background-color: rgb(var(--color-app-panel));
          border: 1px solid rgb(var(--color-app-border));
          border-radius: 6px;
        }
        .wiki-markdown pre code {
          padding: 0;
          margin: 0;
          font-size: 100%;
          word-break: normal;
          white-space: pre;
          background: transparent;
          border: 0;
        }
        .wiki-markdown blockquote {
          padding: 0 1em;
          color: rgb(var(--color-app-muted));
          border-left: 0.25em solid rgb(var(--color-app-border));
        }
        .wiki-markdown hr {
          height: 0.25em;
          padding: 0;
          margin: 24px 0;
          background-color: rgb(var(--color-app-border));
          border: 0;
        }
        .wiki-markdown table {
          display: block;
          width: 100%;
          width: max-content;
          max-width: 100%;
          overflow: auto;
          border-collapse: collapse;
        }
        .wiki-markdown table th {
          font-weight: 600;
        }
        .wiki-markdown table th,
        .wiki-markdown table td {
          padding: 6px 13px;
          border: 1px solid rgb(var(--color-app-border));
        }
        .wiki-markdown table tr {
          background-color: transparent;
          border-top: 1px solid rgb(var(--color-app-border));
        }
        .wikilink {
          color: rgb(var(--color-app-accent));
          font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace;
          background-color: rgba(var(--color-app-accent), 0.1);
          padding: 0.1em 0.3em;
          border-radius: 4px;
          font-size: 0.9em;
          cursor: pointer;
        }
        .wikilink:hover {
          text-decoration: underline;
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
    { label: 'Setup & Installation', ok: /install|setup|run|start/.test(value) },
    { label: 'Architecture & Design', ok: /architecture|flow|design|module/.test(value) },
    { label: 'API & Endpoints', ok: /api|endpoint|route|http/.test(value) },
    { label: 'Debugging & Issues', ok: /error|debug|troubleshoot|issue/.test(value) },
  ];
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

function wikiFileName(path: string | null) {
  if (!path) return '';
  return `${path.replace(/[\\/]/g, '_')}.md`;
}
