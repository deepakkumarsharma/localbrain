import type { FormEvent } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { ChevronDown, FileCode2, Info, Link2, MessageSquare, Send, Settings } from 'lucide-react';
import { marked } from 'marked';
import { getAgentApiStatus } from '../lib/api';
import type { ChatMessage, Citation } from '../lib/chat';
import { askLocal } from '../lib/chat';
import type { GraphContext } from '../lib/graph';
import { getGraphContext } from '../lib/graph';
import { getProviderSettings } from '../lib/settings';
import { useAppStore } from '../store/useAppStore';

export function RightPanel() {
  const {
    activeSourcePath,
    agentApiStatus,
    appVersion,
    chatMessages,
    graphContext,
    selectedGraphNode,
    providerSettings,
    llmRunning,
    projectPath,
    setActiveSourcePath,
    addChatMessage,
    replaceChatMessage,
    setAgentApiStatus,
    setChatError,
    setCitations,
    setGraphContext,
    setGraphError,
    setProviderSettings,
  } = useAppStore();
  const [query, setQuery] = useState('');
  const [isAsking, setIsAsking] = useState(false);
  const [hasAskedForCurrentFile, setHasAskedForCurrentFile] = useState(false);
  const [relationshipsCollapsed, setRelationshipsCollapsed] = useState(false);
  const chatRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    void getProviderSettings().then(setProviderSettings).catch(console.error);
    void getAgentApiStatus().then(setAgentApiStatus).catch(console.error);
  }, [setAgentApiStatus, setProviderSettings]);

  useEffect(() => {
    if (!projectPath || !activeSourcePath) {
      setGraphContext([]);
      return;
    }
    const controller = new AbortController();
    void getGraphContext(activeSourcePath, 12)
      .then((context) => {
        if (!controller.signal.aborted) {
          setGraphContext(normalizeGraphContext(context));
        }
      })
      .catch((error) => {
        if (!controller.signal.aborted) {
          setGraphError(error instanceof Error ? error.message : String(error));
        }
      });
    return () => controller.abort();
  }, [activeSourcePath, projectPath, setGraphContext, setGraphError]);

  useEffect(() => {
    setHasAskedForCurrentFile(false);
  }, [activeSourcePath]);

  useEffect(() => {
    const element = chatRef.current;
    if (element) {
      element.scrollTop = element.scrollHeight;
    }
  }, [chatMessages.length, isAsking]);

  async function handleAsk(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await askQuestion(query);
  }

  async function askQuestion(value: string, sourcePath?: string) {
    const trimmed = value.trim();
    if (!trimmed || isAsking || !projectPath) {
      return;
    }

    setChatError(null);
    setIsAsking(true);
    setHasAskedForCurrentFile(true);
    setQuery('');
    const userMessage = createChatMessage('user', trimmed);
    const pendingId = `assistant-${Date.now()}`;
    const pendingMessage = {
      id: pendingId,
      role: 'assistant' as const,
      content: 'Thinking...',
      citations: [],
      createdAt: Date.now(),
      status: 'pending' as const,
    };
    addChatMessage(userMessage);
    addChatMessage(pendingMessage);

    const thinkingSteps = [
      'Searching Wiki Memory...',
      'Querying Code Graph...',
      'Analyzing local index...',
      'Assembling answer...',
    ];

    try {
      for (const step of thinkingSteps) {
        replaceChatMessage(pendingId, { ...pendingMessage, content: step });
        await new Promise((resolve) => setTimeout(resolve, 600));
      }

      const resolvedSourcePath = sourcePath === undefined ? activeSourcePath : sourcePath;
      const answer = await askLocal(trimmed, resolvedSourcePath);
      const citations = normalizeCitations(answer.citations);
      const graphContext = normalizeGraphContext(answer.graphContext);
      const content =
        typeof answer.answer === 'string' && answer.answer.trim()
          ? answer.answer
          : 'No local answer was returned. Try again after the index finishes refreshing.';
      setCitations(citations);
      setGraphContext(graphContext);
      replaceChatMessage(pendingId, {
        ...pendingMessage,
        content,
        citations,
        status: 'complete',
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setChatError(message);
      replaceChatMessage(pendingId, {
        ...pendingMessage,
        content: message,
        status: 'error',
      });
    } finally {
      setIsAsking(false);
    }
  }

  async function handleEvidenceClick(citation: Citation) {
    if (!projectPath || isAsking) {
      return;
    }

    if (citation.path) {
      setActiveSourcePath(citation.path);
    }

    const lineScope = citation.startLine
      ? citation.endLine && citation.endLine !== citation.startLine
        ? `L${citation.startLine}-L${citation.endLine}`
        : `L${citation.startLine}`
      : 'relevant lines';

    await askQuestion(
      `Explain ${citation.path} (${lineScope}) in detail: intent, data flow, key logic, risks, and what to change safely.`,
      citation.path,
    );
  }

  const focusedName = projectPath
    ? (selectedGraphNode?.label ?? sourceFileName(activeSourcePath || 'No file selected'))
    : 'No project selected';
  const focusedKind = projectPath ? (selectedGraphNode?.kind ?? 'Feature') : 'Idle';
  const isLocalReady = Boolean(providerSettings?.localModelPath) && llmRunning;
  const canAskLocalBrain = Boolean(projectPath) && !isAsking;
  const knowledgeCachedLocally = Boolean(projectPath);
  const isReady = isLocalReady && knowledgeCachedLocally;
  const safeGraphContext = useMemo(() => normalizeGraphContext(graphContext), [graphContext]);
  const dependentCount = safeGraphContext.length;
  const exportCount = Math.max(
    0,
    safeGraphContext.filter((item) => item.relation.toLowerCase().includes('contains')).length,
  );
  const summaryText = projectPath
    ? activeSourcePath
      ? `Indexed context for ${sourceFileName(activeSourcePath)} is available with ${dependentCount} related symbols.`
      : 'Select a file to see enriched graph details.'
    : 'Load a project to build enriched node context.';
  const purposeText = activeSourcePath
    ? inferPurposeFromPath(activeSourcePath)
    : 'No active file selected';
  const complexity = dependentCount > 8 ? 'High' : dependentCount > 3 ? 'Medium' : 'Low';
  const suggestedQuestions = useMemo(
    () =>
      buildSuggestedQuestions(
        activeSourcePath,
        selectedGraphNode?.label,
        selectedGraphNode?.kind,
      ).slice(0, 3),
    [activeSourcePath, selectedGraphNode?.kind, selectedGraphNode?.label],
  );
  return (
    <aside className="flex h-full min-w-0 flex-col border-l border-app-border bg-app-panel">
      <div className="border-b border-app-border p-4">
        <h3 className="flex items-center gap-2 text-[13px] font-black uppercase tracking-widest text-app-muted">
          <Settings className="h-4 w-4" aria-hidden="true" />
          KNOWLEDGE GRAPH
        </h3>
      </div>

      <div className="space-y-3 border-b border-app-border bg-app-panel/20 p-3 max-h-[33%] overflow-auto">
        {projectPath ? (
          <>
            <div className="relative overflow-hidden rounded-2xl border border-app-border bg-app-background p-3.5 shadow-sm">
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 flex items-start gap-3">
                  <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-app-border bg-app-panelSoft/60">
                    <FileCode2 className="h-5 w-5 text-app-muted" aria-hidden="true" />
                  </div>
                  <div className="min-w-0">
                    <div
                      className="truncate text-[21px] font-black tracking-[-0.02em] text-app-text"
                      title={focusedName}
                    >
                      {focusedName}
                    </div>
                    <div className="mt-1.5 flex items-center gap-2">
                      <span className="inline-flex items-center rounded-full border border-app-border bg-app-panelSoft px-2.5 py-0.5 text-[10px] font-black uppercase tracking-[0.11em] text-app-text">
                        {focusedKind}
                      </span>
                      <span className="text-[10px] font-bold uppercase tracking-[0.09em] text-app-muted">
                        {providerSettings?.provider ?? 'local'} · v{appVersion}
                      </span>
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2 rounded-xl border border-app-border bg-app-panelSoft px-2.5 py-1.5 text-right">
                  <div className="text-[9px] font-black uppercase tracking-[0.11em] text-app-muted">
                    Cloud
                  </div>
                  <span
                    className={`relative h-5 w-9 rounded-full border transition-colors ${providerSettings?.cloudEnabled ? 'border-app-accent bg-app-accent/35' : 'border-app-border bg-app-background'}`}
                  >
                    <span
                      className={`absolute top-[1px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${providerSettings?.cloudEnabled ? 'translate-x-4' : 'translate-x-0.5'}`}
                    />
                  </span>
                  <div className="mt-0.5 text-[11px] font-extrabold text-app-text">
                    {providerSettings?.cloudEnabled ? 'On' : 'Off'}
                  </div>
                </div>
              </div>
              <div className="mt-3 grid grid-cols-2 gap-2">
                <MetricPill label="Dependents" value={projectPath ? String(dependentCount) : '0'} />
                <MetricPill label="Exports" value={String(exportCount)} />
                <MetricPill label="Complexity" value={complexity} />
                <MetricPill label="Status" value={projectPath ? 'Live' : 'Idle'} />
              </div>
              <div className="mt-2.5 rounded-xl border border-app-border bg-app-panelSoft/50 p-2">
                <div className="grid grid-cols-[auto,1fr] items-center gap-2">
                  <span className="text-app-muted font-black text-[9px] uppercase tracking-[0.11em]">
                    File
                  </span>
                  <span className="flex items-center justify-end gap-1.5 text-[12px] font-semibold text-app-text">
                    <span className="truncate font-mono">
                      {projectPath ? activeSourcePath || '-' : '-'}
                    </span>
                    <Link2 className="h-3.5 w-3.5 text-app-muted" aria-hidden="true" />
                  </span>
                </div>
              </div>
            </div>

            <div className="rounded-2xl border border-app-border bg-app-background p-3.5 shadow-sm">
              <div className="mb-2 text-[10px] font-black uppercase tracking-[0.12em] text-app-muted">
                Knowledge Graph Enrichment
              </div>
              <div className="mb-2 flex items-start gap-2 rounded-lg border border-app-border bg-app-panelSoft/40 px-2.5 py-2 text-[12px] leading-6 text-app-text/95">
                <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-app-muted" aria-hidden="true" />
                <p>{summaryText}</p>
              </div>
              <div className="grid grid-cols-2 gap-x-4 gap-y-1.5 text-[12px]">
                <InfoRow label="Purpose" value={purposeText} />
                <InfoRow label="Dependents" value={String(dependentCount)} />
                <InfoRow label="Last Author" value="Pending git integration" />
                <InfoRow label="Test Status" value="Pending coverage mapping" />
              </div>
            </div>

            <div>
              <button
                className="mb-1.5 flex w-full items-center justify-between rounded-lg px-1 py-1 text-left hover:bg-app-panelSoft/50"
                type="button"
                onClick={() => setRelationshipsCollapsed((current) => !current)}
              >
                <span className="text-[10px] font-black uppercase tracking-[0.12em] text-app-muted">
                  RELATIONSHIPS
                </span>
                <ChevronDown
                  className={`h-4 w-4 text-app-muted transition-transform ${relationshipsCollapsed ? '-rotate-90' : 'rotate-0'}`}
                  aria-hidden="true"
                />
              </button>
              <div className={`${relationshipsCollapsed ? 'hidden' : 'pt-0.5'}`}>
                <span className="text-[12px] text-app-muted font-medium italic">
                  {!projectPath
                    ? 'Load a project to see relationships'
                    : safeGraphContext.length === 0
                      ? 'No graph context loaded'
                      : `${safeGraphContext
                          .slice(0, 4)
                          .map((item) => item.symbol.name)
                          .join(', ')}${safeGraphContext.length > 4 ? '…' : ''}`}
                </span>
              </div>
            </div>
          </>
        ) : (
          <div className="rounded-2xl border border-app-border bg-app-background p-4 shadow-sm">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-xl border border-app-border bg-app-panelSoft/60">
                <FileCode2 className="h-5 w-5 text-app-muted" aria-hidden="true" />
              </div>
              <div>
                <div className="text-[18px] font-black tracking-tight text-app-text">
                  No project selected
                </div>
                <div className="mt-0.5 text-[12px] text-app-muted">
                  Select a project folder from the left panel to load Knowledge Graph context.
                </div>
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="min-h-0 flex-1 bg-app-panel/40 flex flex-col">
        <div className="border-b border-app-border px-4 py-3">
          <div className="mb-3 flex items-center justify-between">
            <h4 className="flex items-center gap-2 text-[13px] font-black uppercase tracking-widest text-app-muted">
              <MessageSquare className="h-4 w-4" aria-hidden="true" />
              ASK LOCAL BRAIN
            </h4>
            <span className="rounded-full border border-app-border bg-app-panelSoft px-2.5 py-1 text-[11px] font-black text-app-text uppercase">
              {agentApiStatus?.running ? 'api on' : 'local'}
            </span>
          </div>
        </div>

        <div className="min-h-0 flex-1 p-4">
          <div ref={chatRef} className="app-scrollbar h-full space-y-3.5 overflow-auto pr-1">
            {chatMessages.length === 0 ? (
              <div className="rounded-2xl border border-app-border bg-app-background p-4 text-[13px] font-medium text-app-muted leading-relaxed shadow-sm">
                {projectPath
                  ? 'Ask focused questions about the selected file, graph node, or dependencies.'
                  : 'Select a project folder from the left panel to unlock Ask Local Brain.'}
              </div>
            ) : (
              chatMessages.map((message) => (
                <div key={message.id} className={message.role === 'user' ? 'flex justify-end' : ''}>
                  <div
                    className={
                      message.role === 'user'
                        ? 'max-w-[85%] rounded-2xl bg-app-accent px-4 py-2.5 text-[14px] font-bold text-white shadow-lg shadow-app-accent/20'
                        : 'max-w-[95%] rounded-2xl border border-app-border bg-app-background px-4 py-3 text-[14px] font-medium text-app-text leading-relaxed shadow-sm'
                    }
                  >
                    {message.role === 'assistant' ? (
                      <>
                        <div
                          className="chat-markdown text-[14px] leading-8 tracking-[0.01em] [&_h1]:mb-3 [&_h1]:mt-1 [&_h1]:text-[17px] [&_h1]:font-black [&_h1]:tracking-tight [&_h1]:text-app-accent [&_h2]:mb-2 [&_h2]:mt-5 [&_h2]:text-[12px] [&_h2]:font-black [&_h2]:uppercase [&_h2]:tracking-[0.12em] [&_h2]:text-app-accent [&_h3]:mb-2 [&_h3]:mt-4 [&_h3]:text-[14px] [&_h3]:font-bold [&_h3]:text-app-text [&_strong]:font-extrabold [&_strong]:text-app-text [&_p]:mb-3 [&_p]:text-app-text/95 [&_ul]:mb-4 [&_ul]:list-disc [&_ul]:space-y-1.5 [&_ul]:pl-5 [&_ol]:mb-4 [&_ol]:list-decimal [&_ol]:space-y-1.5 [&_ol]:pl-5 [&_li]:text-app-text/90 [&_li]:leading-7 [&_blockquote]:my-3 [&_blockquote]:border-l-2 [&_blockquote]:border-app-accent [&_blockquote]:pl-3 [&_blockquote]:text-app-muted [&_code]:rounded [&_code]:border [&_code]:border-app-border [&_code]:bg-app-panelSoft [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-[12px] [&_pre]:mb-4 [&_pre]:overflow-auto [&_pre]:rounded-lg [&_pre]:border [&_pre]:border-app-border [&_pre]:bg-app-panelSoft [&_pre]:p-3 [&_table]:my-3 [&_table]:block [&_table]:w-full [&_table]:max-w-[min(92vw,100%)] [&_table]:overflow-x-auto [&_table]:border-collapse [&_table]:rounded-lg [&_table]:border [&_table]:border-app-border [&_th]:whitespace-nowrap [&_th]:bg-app-panelSoft [&_th]:px-3 [&_th]:py-2 [&_th]:text-left [&_th]:text-[12px] [&_th]:font-extrabold [&_td]:min-w-[140px] [&_td]:px-3 [&_td]:py-2 [&_td]:align-top [&_td]:text-[13px]"
                          dangerouslySetInnerHTML={{
                            __html: sanitizeHtml(
                              marked.parse(formatAssistantResponse(message.content), {
                                async: false,
                              }) as string,
                            ),
                          }}
                        />
                        {message.citations?.length > 0 ? (
                          <div className="mt-3 border-t border-app-border pt-2">
                            <div className="mb-1 text-[10px] font-black uppercase tracking-widest text-app-muted">
                              Evidence
                            </div>
                            <div className="flex flex-wrap gap-1.5">
                              {message.citations.slice(0, 4).map((citation, index) => (
                                <button
                                  key={`${citation.path}-${index}`}
                                  className="inline-flex items-center gap-1.5 rounded-full border border-app-border bg-app-panelSoft px-2 py-1 text-[11px] font-bold text-app-text hover:border-app-accent hover:text-app-accent transition-colors"
                                  title={sourceLabel(citation)}
                                  type="button"
                                  onClick={() => void handleEvidenceClick(citation)}
                                >
                                  <span className="font-mono">
                                    {sourcePathLabel(citation.path)}
                                  </span>
                                  {citation.startLine ? (
                                    <span className="inline-flex items-center rounded-full border border-app-border bg-app-accent/15 px-1.5 py-0.5 font-mono text-[10px] font-extrabold tracking-[0.04em] text-app-accent">
                                      {lineChipLabel(citation.startLine, citation.endLine)}
                                    </span>
                                  ) : null}
                                </button>
                              ))}
                            </div>
                          </div>
                        ) : null}
                      </>
                    ) : (
                      <p className="whitespace-pre-wrap">{message.content}</p>
                    )}
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
        <div className="border-t border-app-border p-4 bg-app-panel">
          {!hasAskedForCurrentFile ? (
            <div className="mb-2 flex flex-wrap gap-2">
              {suggestedQuestions.map((question) => (
                <button
                  key={question}
                  className="rounded-full border border-app-border bg-app-panelSoft px-3 py-1 text-[11px] font-semibold text-app-muted hover:text-app-text hover:border-app-accent transition-all"
                  type="button"
                  disabled={!canAskLocalBrain}
                  onClick={() => void askQuestion(question)}
                >
                  {question}
                </button>
              ))}
            </div>
          ) : null}
          <form className="relative" onSubmit={handleAsk}>
            <input
              className="h-11 w-full rounded-xl border border-app-border bg-app-background pl-4 pr-11 text-[14px] font-bold outline-none placeholder:text-app-muted focus:ring-2 focus:ring-app-accent focus:border-transparent transition-all shadow-inner"
              placeholder={
                !projectPath
                  ? 'Select a project folder first...'
                  : isLocalReady
                    ? 'Ask about this file or node...'
                    : 'Ask from the local index...'
              }
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              disabled={!canAskLocalBrain}
            />
            <button
              className="absolute right-2 top-2 rounded-lg p-1.5 text-app-muted hover:bg-app-panelSoft hover:text-app-text transition-colors"
              type="submit"
              disabled={!canAskLocalBrain}
              aria-label="Ask Localbrain"
            >
              <Send className="h-5 w-5" aria-hidden="true" />
            </button>
          </form>
        </div>
      </div>
      <div className="flex items-center gap-3 border-t border-app-border px-3 py-3 text-[9px] font-extrabold uppercase tracking-[0.07em] text-app-muted whitespace-nowrap">
        <span className="inline-flex items-center gap-1.5">
          <span
            className={`h-2 w-2 rounded-full ${isReady ? 'bg-app-success animate-pulse shadow-[0_0_8px_rgba(var(--color-app-success),0.45)]' : 'bg-app-error shadow-[0_0_6px_rgba(var(--color-app-error),0.35)]'}`}
          />
          <span className={isReady ? 'text-app-success' : 'text-app-error'}>Ready</span>
        </span>
        <span className="inline-flex items-center gap-1.5">
          <span
            className={`h-2 w-2 rounded-full ${knowledgeCachedLocally ? 'bg-app-success shadow-[0_0_8px_rgba(var(--color-app-success),0.45)]' : 'bg-app-error shadow-[0_0_6px_rgba(var(--color-app-error),0.35)]'}`}
          />
          <span className={knowledgeCachedLocally ? 'text-app-success' : 'text-app-error'}>
            Knowledge Cached Locally
          </span>
        </span>
        <span className="inline-flex items-center gap-1.5">
          <span
            className={`h-2 w-2 rounded-full ${isLocalReady ? 'bg-app-success shadow-[0_0_8px_rgba(var(--color-app-success),0.45)]' : 'bg-app-error shadow-[0_0_6px_rgba(var(--color-app-error),0.35)]'}`}
          />
          <span className={isLocalReady ? 'text-app-success' : 'text-app-error'}>
            Local Model Is Ready
          </span>
        </span>
      </div>
    </aside>
  );
}

function InfoRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="grid grid-cols-[auto,1fr] items-start gap-x-2 gap-y-0.5 py-0.5">
      <span className="text-app-muted font-black text-[9px] uppercase tracking-[0.11em]">
        {label}
      </span>
      <span
        className={`text-right text-app-text ${mono ? 'truncate font-mono text-[12px] font-medium' : 'whitespace-normal break-words font-semibold text-[12px]'}`}
      >
        {value}
      </span>
    </div>
  );
}

function MetricPill({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-app-border bg-app-panelSoft px-2.5 py-1.5">
      <div className="text-[9px] font-black uppercase tracking-[0.1em] text-app-muted">{label}</div>
      <div className="mt-0.5 truncate text-[13px] font-extrabold text-app-text">{value}</div>
    </div>
  );
}

function inferPurposeFromPath(path: string) {
  const lower = path.toLowerCase();
  if (lower.includes('workflow') || lower.includes('github')) return 'Automation / CI behavior';
  if (lower.includes('test')) return 'Validation and quality checks';
  if (lower.includes('component')) return 'UI rendering and interaction';
  if (lower.includes('parser') || lower.includes('index')) return 'Code intelligence and indexing';
  return 'General project logic';
}

function buildSuggestedQuestions(
  activePath: string,
  selectedNodeLabel?: string,
  selectedNodeKind?: string,
) {
  const file = sourceFileName(activePath || 'this file');
  const node = selectedNodeLabel || file;
  const nodeKind = (selectedNodeKind || 'symbol').split('_').join(' ');

  const suggestions = [
    `What does ${file} do?`,
    `Explain ${node} and its dependencies`,
    `Where is ${node} used?`,
    `Show call flow for ${node}`,
    `Summarize risks and edge cases in ${file}`,
    `What tests cover this ${nodeKind}?`,
  ];

  const unique = new Set<string>();
  return suggestions.filter((item) => {
    if (unique.has(item)) return false;
    unique.add(item);
    return true;
  });
}

function sourceFileName(path: string) {
  return path.split('/').pop() ?? path;
}

function sourceLabel(source: { path: string; startLine: number | null; endLine: number | null }) {
  if (source.startLine && source.endLine) {
    return source.startLine === source.endLine
      ? `${source.path}:L${source.startLine}`
      : `${source.path}:L${source.startLine}-L${source.endLine}`;
  }
  return source.path;
}

function sourcePathLabel(path: string) {
  return path.length > 52 ? `...${path.slice(-49)}` : path;
}

function lineChipLabel(startLine: number, endLine: number | null) {
  if (endLine && endLine !== startLine) {
    return `L${startLine} - L${endLine}`;
  }
  return `L${startLine}`;
}

function createChatMessage(role: ChatMessage['role'], content: string): ChatMessage {
  return {
    id: `${role}-${Date.now()}-${Math.random().toString(36).slice(2)}`,
    role,
    content,
    citations: [],
    createdAt: Date.now(),
    status: role === 'assistant' ? 'pending' : 'complete',
  };
}

function normalizeCitations(value: unknown): Citation[] {
  if (!Array.isArray(value)) return [];

  return value.flatMap((item) => {
    if (!item || typeof item !== 'object') return [];
    const citation = item as Partial<Citation>;
    if (typeof citation.path !== 'string' || citation.path.length === 0) return [];

    return [
      {
        path: citation.path,
        title: typeof citation.title === 'string' ? citation.title : sourceFileName(citation.path),
        snippet: typeof citation.snippet === 'string' ? citation.snippet : '',
        startLine: typeof citation.startLine === 'number' ? citation.startLine : null,
        endLine: typeof citation.endLine === 'number' ? citation.endLine : null,
        score: typeof citation.score === 'number' ? citation.score : 0,
      },
    ];
  });
}

function normalizeGraphContext(value: unknown): GraphContext[] {
  if (!Array.isArray(value)) return [];

  return value.flatMap((item) => {
    if (!item || typeof item !== 'object') return [];
    const context = item as Partial<GraphContext>;
    if (typeof context.path !== 'string' || context.path.length === 0) return [];
    const symbol = context.symbol;

    return [
      {
        path: context.path,
        relation: typeof context.relation === 'string' ? context.relation : 'related',
        symbol: {
          name:
            symbol && typeof symbol.name === 'string' ? symbol.name : sourceFileName(context.path),
          kind: symbol?.kind ?? 'function',
          parent: symbol?.parent ?? null,
          source: symbol?.source ?? null,
          range: {
            startLine: typeof symbol?.range?.startLine === 'number' ? symbol.range.startLine : 1,
            startColumn:
              typeof symbol?.range?.startColumn === 'number' ? symbol.range.startColumn : 0,
            endLine: typeof symbol?.range?.endLine === 'number' ? symbol.range.endLine : 1,
            endColumn: typeof symbol?.range?.endColumn === 'number' ? symbol.range.endColumn : 0,
          },
        },
      },
    ];
  });
}

function sanitizeHtml(value: string): string {
  const parser = new DOMParser();
  const doc = parser.parseFromString(value, 'text/html');
  const allowedTags = new Set([
    'p',
    'strong',
    'em',
    'h1',
    'h2',
    'h3',
    'h4',
    'h5',
    'h6',
    'ul',
    'ol',
    'li',
    'blockquote',
    'code',
    'pre',
    'table',
    'thead',
    'tbody',
    'tr',
    'th',
    'td',
    'hr',
    'br',
    'div',
    'span',
  ]);

  doc.body.querySelectorAll('script, style, iframe, object, embed, link, meta').forEach((node) => {
    node.remove();
  });

  doc.body.querySelectorAll('a, img').forEach((node) => {
    const replacement = doc.createElement('span');
    replacement.textContent = node.textContent ?? '';
    node.replaceWith(replacement);
  });

  doc.body.querySelectorAll<HTMLElement>('*').forEach((element) => {
    const tag = element.tagName.toLowerCase();
    if (!allowedTags.has(tag)) {
      element.replaceWith(...Array.from(element.childNodes));
      return;
    }
    const attrs = Array.from(element.attributes);
    for (const attr of attrs) {
      const name = attr.name.toLowerCase();
      const val = attr.value.trim().toLowerCase();
      if (
        name.startsWith('on') ||
        name === 'href' ||
        name === 'src' ||
        val.startsWith('javascript:')
      ) {
        element.removeAttribute(attr.name);
      }
    }
  });

  return doc.body.innerHTML;
}

function formatAssistantResponse(value: string) {
  const text = value.replace(/\r\n/g, '\n').trim();
  if (!text) return text;

  const normalized = text
    .split('\n')
    .map((line) => line.trimEnd())
    .join('\n')
    .replace(/([.!?])\n(?=[A-Z#-])/g, '$1\n\n');

  const lines = normalized.split('\n');
  const withHeaders = lines.map((line) => {
    const trimmed = line.trim();
    if (!trimmed) return '';
    if (/^#{1,6}\s/.test(trimmed) || /^[-*]\s/.test(trimmed) || /^\d+\.\s/.test(trimmed)) {
      return line;
    }
    if (/^[A-Za-z][A-Za-z\s]+:\s*$/.test(trimmed)) {
      return `## ${trimmed.slice(0, -1)}`;
    }
    return line;
  });

  return withHeaders.join('\n');
}
