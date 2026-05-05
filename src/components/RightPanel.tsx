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
  }, [activeSourcePath, setGraphContext, setGraphError]);

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
    if (!trimmed || isAsking) {
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

  const focusedName = selectedGraphNode?.label ?? sourceFileName(activeSourcePath);
  const focusedKind = selectedGraphNode?.kind ?? 'Feature';

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
          <InfoRow label="File" value={activeSourcePath} mono />
          <InfoRow
            label="References"
            value={String(graphContext.length || citations.length || 0)}
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
            {graphContext.length === 0 ? (
              <span className="text-[12px] text-app-muted font-medium italic">
                No graph context loaded
              </span>
            ) : null}
          </div>
        </div>
      </div>

      <div className="app-scrollbar min-h-0 flex-1 overflow-y-auto border-b border-app-border p-4">
        <h4 className="mb-3 flex items-center justify-between text-[13px] font-black uppercase tracking-widest text-app-muted">
          SOURCES
          <span className="text-[11px] font-bold bg-app-panelSoft px-2 py-0.5 rounded-md border border-app-border">
            {citations.length || 0} files
          </span>
        </h4>
        <div className="space-y-2">
          {(citations.length > 0 ? citations : fallbackSources(activeSourcePath))
            .slice(0, 6)
            .map((source, index) => (
              <div
                key={`${source.path}-${index}`}
                className="flex items-center justify-between gap-3 rounded-xl border border-app-border bg-app-background p-3 hover:border-app-accent/50 transition-colors cursor-pointer group"
              >
                <span
                  className="min-w-0 truncate font-mono text-[13px] font-medium text-app-muted group-hover:text-app-text transition-colors"
                  title={source.path}
                >
                  {source.path}
                </span>
                <span className="rounded-lg bg-app-panelSoft px-2 py-1 text-[11px] font-bold text-app-muted group-hover:bg-app-accent group-hover:text-white transition-all uppercase tracking-tighter">
                  {source.title ? 'code' : 'source'}
                </span>
              </div>
            ))}
        </div>
      </div>

      <div className="p-4 bg-app-panel/40">
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
              <span className="mr-2.5 inline-block h-2.5 w-2.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]" />
              Local Brain is ready. Ask anything about this workspace.
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
                  <p className="whitespace-pre-wrap">{message.content}</p>
                </div>
              </div>
            ))
          )}
        </div>
        <form className="relative" onSubmit={handleAsk}>
          <input
            className="h-11 w-full rounded-xl border border-app-border bg-app-background pl-4 pr-11 text-[14px] font-bold outline-none placeholder:text-app-muted focus:ring-2 focus:ring-app-accent focus:border-transparent transition-all shadow-inner"
            placeholder="Ask about the codebase..."
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <button
            className="absolute right-2 top-2 rounded-lg p-1.5 text-app-muted hover:bg-app-panelSoft hover:text-app-text transition-colors"
            type="submit"
            disabled={isAsking}
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

function fallbackSources(path: string) {
  return [
    { path, title: sourceFileName(path), snippet: '', score: 1 },
    { path: 'docs/wiki/index.md', title: 'wiki', snippet: '', score: 1 },
  ];
}

function sourceFileName(path: string) {
  return path.split('/').pop() ?? path;
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
