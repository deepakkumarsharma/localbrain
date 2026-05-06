import type { FormEvent } from 'react';
import { useEffect, useRef, useState } from 'react';
import { MessageSquare, Send, Settings } from 'lucide-react';
import { getAgentApiStatus } from '../lib/api';
import type { ChatMessage } from '../lib/chat';
import { askLocal } from '../lib/chat';
import { getGraphContext } from '../lib/graph';
import { getProviderSettings } from '../lib/settings';
import { useAppStore } from '../store/useAppStore';

const suggestedQuestions = ['How does this work?', 'Where is graph logic?', 'Show parser flow'];

export function RightPanel() {
  const {
    activeSourcePath,
    agentApiStatus,
    appVersion,
    chatMessages,
    citations,
    graphContext,
    selectedGraphNode,
    providerSettings,
    llmRunning,
    projectPath,
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
          setGraphContext(context);
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
    const element = chatRef.current;
    if (element) {
      element.scrollTop = element.scrollHeight;
    }
  }, [chatMessages.length, isAsking]);

  async function handleAsk(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await askQuestion(query);
  }

  async function askQuestion(value: string) {
    const trimmed = value.trim();
    if (!trimmed || isAsking || !projectPath) {
      return;
    }

    setChatError(null);
    setIsAsking(true);
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

      const answer = await askLocal(trimmed);
      setCitations(answer.citations);
      setGraphContext(answer.graphContext);
      replaceChatMessage(pendingId, {
        ...pendingMessage,
        content: answer.answer,
        citations: answer.citations,
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

  const focusedName = projectPath
    ? (selectedGraphNode?.label ?? sourceFileName(activeSourcePath || 'No file selected'))
    : 'No project selected';
  const focusedKind = projectPath ? (selectedGraphNode?.kind ?? 'Feature') : 'Idle';
  const isLocalReady = Boolean(providerSettings?.localModelPath) && llmRunning;
  const canAskLocalBrain = Boolean(projectPath) && !isAsking;
  return (
    <aside className="flex h-full min-w-0 flex-col border-l border-app-border bg-app-panel">
      <div className="border-b border-app-border p-4">
        <h3 className="flex items-center gap-2 text-[13px] font-black uppercase tracking-widest text-app-muted">
          <Settings className="h-4 w-4" aria-hidden="true" />
          KNOWLEDGE GRAPH
        </h3>
      </div>

      <div className="space-y-4 border-b border-app-border p-4 bg-app-panel/30">
        <div className="flex items-start justify-between">
          <div className="min-w-0">
            <div className="truncate text-[18px] font-bold text-app-text" title={focusedName}>
              {focusedName}
            </div>
            <div className="mt-2 inline-flex items-center gap-1.5 rounded-full border border-blue-500/30 bg-blue-500/10 px-3 py-1 text-[12px] font-bold text-blue-400">
              {focusedKind}
            </div>
          </div>
        </div>
        <div className="space-y-2.5 text-[14px] font-medium">
          <InfoRow label="File" value={projectPath ? activeSourcePath || '-' : '-'} mono />
          <InfoRow
            label="References"
            value={projectPath ? String(graphContext.length || citations.length || 0) : '0'}
          />
          <InfoRow
            label="Provider"
            value={`${providerSettings?.provider ?? 'local'} · cloud ${providerSettings?.cloudEnabled ? 'on' : 'off'}`}
          />
          <InfoRow label="Version" value={appVersion} />
        </div>
        <div>
          <div className="mb-2 text-[11px] font-black tracking-widest text-app-muted">
            RELATIONSHIPS
          </div>
          <div className="flex flex-wrap gap-2">
            {graphContext.slice(0, 4).map((item) => (
              <span
                key={`${item.path}-${item.symbol.name}-${item.symbol.range.startLine}`}
                className="rounded-lg border border-app-border bg-app-background px-2.5 py-1 text-[12px] font-bold text-violet-400 shadow-sm"
              >
                {item.symbol.name}
              </span>
            ))}
            {!projectPath || graphContext.length === 0 ? (
              <span className="text-[12px] text-app-muted font-medium italic">
                {projectPath ? 'No graph context loaded' : 'Load a project to see relationships'}
              </span>
            ) : null}
          </div>
        </div>
      </div>

      <div className="min-h-0 flex-1 p-4 bg-app-panel/40 flex flex-col">
        <div className="mb-3 flex items-center justify-between">
          <h4 className="flex items-center gap-2 text-[13px] font-black uppercase tracking-widest text-app-muted">
            <MessageSquare className="h-4 w-4" aria-hidden="true" />
            ASK LOCAL BRAIN
          </h4>
          <span className="rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-1 text-[11px] font-black text-emerald-400 uppercase">
            {agentApiStatus?.running ? 'api on' : 'local'}
          </span>
        </div>
        <div
          ref={chatRef}
          className="app-scrollbar mb-4 max-h-[220px] space-y-3.5 overflow-auto pr-1"
        >
          {chatMessages.length === 0 ? (
            <div className="rounded-xl border border-app-border bg-app-background p-4 text-[14px] font-medium text-app-muted leading-relaxed">
              <span
                className={`mr-2.5 inline-block h-2.5 w-2.5 rounded-full ${isLocalReady ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]' : 'bg-amber-500 shadow-[0_0_8px_rgba(245,158,11,0.5)]'}`}
              />
              {isLocalReady
                ? projectPath
                  ? 'Local model is ready. Ask anything about this workspace.'
                  : 'Select a project folder from the left panel to unlock Ask Local Brain.'
                : 'Local model is not ready yet. Select a model and start server from the left panel.'}
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
                        className="chat-markdown text-[14px] leading-7 [&_h1]:mb-3 [&_h1]:mt-1 [&_h1]:text-[16px] [&_h1]:font-black [&_h1]:tracking-tight [&_h2]:mb-2 [&_h2]:mt-5 [&_h2]:text-[12px] [&_h2]:font-black [&_h2]:uppercase [&_h2]:tracking-[0.08em] [&_h2]:text-app-muted [&_h3]:mb-2 [&_h3]:mt-4 [&_h3]:text-[13px] [&_h3]:font-bold [&_p]:mb-3 [&_p]:text-app-text/95 [&_ul]:mb-3 [&_ul]:list-disc [&_ul]:space-y-1 [&_ul]:pl-5 [&_li]:text-app-text/90 [&_code]:rounded [&_code]:border [&_code]:border-app-border [&_code]:bg-app-panelSoft [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-[12px] [&_pre]:mb-3 [&_pre]:overflow-auto [&_pre]:rounded-lg [&_pre]:border [&_pre]:border-app-border [&_pre]:bg-app-panelSoft [&_pre]:p-3"
                        dangerouslySetInnerHTML={{
                          __html: sanitizeHtml(
                            marked.parse(normalizeAssistantText(message.content)) as string,
                          ),
                        }}
                      />
                      {message.citations.length > 0 ? (
                        <div className="mt-3 border-t border-app-border pt-2">
                          <div className="mb-1 text-[10px] font-black uppercase tracking-widest text-app-muted">
                            Evidence
                          </div>
                          <div className="flex flex-wrap gap-1.5">
                            {message.citations.slice(0, 4).map((citation, index) => (
                              <span
                                key={`${citation.path}-${index}`}
                                className="inline-flex items-center rounded-full border border-app-border bg-app-panelSoft px-2 py-0.5 font-mono text-[11px] text-app-muted"
                                title={sourceLabel(citation)}
                              >
                                {sourceLabel(citation)}
                              </span>
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
        <form className="relative" onSubmit={handleAsk}>
          <input
            className="h-11 w-full rounded-xl border border-app-border bg-app-background pl-4 pr-11 text-[14px] font-bold outline-none placeholder:text-app-muted focus:ring-2 focus:ring-app-accent focus:border-transparent transition-all shadow-inner"
            placeholder={
              !projectPath
                ? 'Select a project folder first...'
                : isLocalReady
                  ? 'Ask about the codebase...'
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
        <div className="mt-3 flex flex-wrap gap-2">
          {suggestedQuestions.map((question) => (
            <button
              key={question}
              className="rounded-full border border-app-border bg-app-panelSoft px-3.5 py-1.5 text-[12px] font-bold text-app-muted hover:text-app-text hover:border-app-accent transition-all"
              type="button"
              disabled={!canAskLocalBrain}
              onClick={() => void askQuestion(question)}
            >
              {question}
            </button>
          ))}
        </div>
      </div>
      <div className="flex items-center justify-between border-t border-app-border px-4 py-3 text-[11px] font-black tracking-widest text-app-muted uppercase">
        <span>Knowledge cached locally</span>
        <span className="flex items-center gap-2">
          <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
          READY
        </span>
      </div>
    </aside>
  );
}

function InfoRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex justify-between gap-4">
      <span className="text-app-muted font-bold text-[12px] uppercase tracking-wider">{label}</span>
      <span
        className={`truncate text-right text-app-text ${mono ? 'font-mono text-[13px]' : 'font-bold'}`}
      >
        {value}
      </span>
    </div>
  );
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

function normalizeAssistantText(value: string) {
  const text = value.replace(/\r\n/g, '\n').trim();
  if (text.includes('\n\n')) {
    return text;
  }
  return text
    .split('\n')
    .map((line) => line.trimEnd())
    .join('\n')
    .replace(/([.!?])\n(?=[A-Z#-])/g, '$1\n\n');
}
