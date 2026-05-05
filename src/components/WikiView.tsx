import { ShieldCheck } from 'lucide-react';
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

    // Apply prototype-style transformations
    const processedHtml = rawHtml
      .replace(/\[\[([^\]]+)\]\]/g, '<span class="wikilink" data-node="$1">$1</span>')
      .replace(/<h1/g, '<h1 class="text-[28px] font-semibold mb-4 text-app-text"')
      .replace(/<h2/g, '<h2 class="text-[18px] font-semibold mt-8 mb-3 text-app-text"')
      .replace(/<p>/g, '<p class="text-[14px] leading-7 text-app-muted my-3">')
      .replace(
        /<code>/g,
        '<code class="px-1.5 py-0.5 rounded bg-app-panelSoft border border-app-border text-[12px] text-app-text font-mono">',
      )
      .replace(
        /<pre>/g,
        '<pre class="my-4 p-4 rounded-lg bg-app-panelSoft border border-app-border overflow-auto text-[12px] leading-6 font-mono app-scrollbar">',
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

  return (
    <div className="absolute inset-0 overflow-auto bg-app-background app-scrollbar">
      <div className="mx-auto max-w-[860px] px-8 py-10">
        <div className="mb-6 flex items-center gap-2">
          <span className="rounded-md border border-app-border bg-app-panel px-2 py-1 font-mono text-[11px] text-app-muted">
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

function wikiFileName(path: string) {
  return `${path.replace(/[\\/]/g, '_')}.md`;
}
