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
    void getGraphContext(activeSourcePath, 12)
      .then(setGraphContext)
      .catch((error) => setGraphError(error instanceof Error ? error.message : String(error)));
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

    setIsAsking(true);
    setQuery('');
    const userMessage = createChatMessage('user', trimmed);
    const pendingMessage = createChatMessage(
      'assistant',
      'Searching wiki memory, graph context, and local index...',
    );
    addChatMessage(userMessage);
    addChatMessage(pendingMessage);

    try {
      const answer = await askLocal(trimmed);
      setCitations(answer.citations);
      setGraphContext(answer.graphContext);
      replaceChatMessage(pendingMessage.id, {
        ...pendingMessage,
        content: answer.answer,
        citations: answer.citations,
        status: 'complete',
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setChatError(message);
      replaceChatMessage(pendingMessage.id, {
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
      <div className="border-b border-app-border p-3">
        <h3 className="flex items-center gap-1.5 text-[12px] font-semibold uppercase tracking-wider text-app-muted">
          <Settings className="h-3.5 w-3.5" aria-hidden="true" />
          Knowledge Graph
        </h3>
      </div>

      <div className="space-y-3 border-b border-app-border p-3">
        <div className="flex items-start justify-between">
          <div className="min-w-0">
            <div className="truncate text-[15px] font-medium">{focusedName}</div>
            <div className="mt-1 inline-flex items-center gap-1 rounded-full border border-blue-500/20 bg-blue-500/10 px-2 py-0.5 text-[11px] font-medium text-blue-400">
              {focusedKind}
            </div>
          </div>
        </div>
        <div className="space-y-2 text-xs">
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
          <div className="mb-1.5 text-[11px] text-app-muted">Relationships</div>
          <div className="flex flex-wrap gap-1.5">
            {graphContext.slice(0, 4).map((item) => (
              <span
                key={`${item.path}-${item.symbol.name}-${item.symbol.range.startLine}`}
                className="rounded-md border border-app-border bg-app-background px-1.5 py-0.5 text-[11px] text-violet-400"
              >
                {item.symbol.name}
              </span>
            ))}
            {graphContext.length === 0 ? (
              <span className="text-[11px] text-app-muted">No graph context loaded</span>
            ) : null}
          </div>
        </div>
      </div>

      <div className="app-scrollbar min-h-0 flex-1 overflow-y-auto border-b border-app-border p-3">
        <h4 className="mb-2 flex items-center justify-between text-[12px] font-semibold uppercase tracking-wider text-app-muted">
          Sources
          <span className="text-[10px] font-normal normal-case">{citations.length || 0} files</span>
        </h4>
        <div className="space-y-1.5">
          {(citations.length > 0 ? citations : fallbackSources(activeSourcePath))
            .slice(0, 6)
            .map((source, index) => (
              <div
                key={`${source.path}-${index}`}
                className="flex items-center justify-between gap-2 rounded-lg border border-app-border bg-app-background p-2"
              >
                <span className="min-w-0 truncate font-mono text-xs text-app-muted">
                  {source.path}
                </span>
                <span className="rounded bg-app-panelSoft px-1.5 py-0.5 text-[10px] text-app-muted">
                  {source.title ? 'code' : 'source'}
                </span>
              </div>
            ))}
        </div>
      </div>

      <div className="p-3">
        <div className="mb-2 flex items-center justify-between">
          <h4 className="flex items-center gap-1.5 text-[12px] font-semibold uppercase tracking-wider text-app-muted">
            <MessageSquare className="h-3.5 w-3.5" aria-hidden="true" />
            Ask Localbrain
          </h4>
          <span className="rounded border border-emerald-500/20 bg-emerald-500/10 px-1.5 py-0.5 text-[10px] text-emerald-400">
            {agentApiStatus?.running ? 'api on' : 'local'}
          </span>
        </div>
        <div
          ref={chatRef}
          className="app-scrollbar mb-2 max-h-[180px] space-y-2.5 overflow-auto pr-1"
        >
          {chatMessages.length === 0 ? (
            <div className="rounded-lg border border-app-border bg-app-background p-2.5 text-xs text-app-muted">
              <span className="mr-1.5 inline-block h-1.5 w-1.5 rounded-full bg-emerald-500" />
              Localbrain ready · Ask anything about this workspace
            </div>
          ) : (
            chatMessages.map((message) => (
              <div key={message.id} className={message.role === 'user' ? 'flex justify-end' : ''}>
                <div
                  className={
                    message.role === 'user'
                      ? 'max-w-[85%] rounded-lg bg-app-panelSoft px-2.5 py-1.5 text-xs'
                      : 'rounded-lg border border-app-border bg-app-background px-2.5 py-2 text-xs leading-5'
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
            className="h-9 w-full rounded-lg border border-app-border bg-app-background pl-3 pr-9 text-[13px] outline-none placeholder:text-app-muted focus:ring-1 focus:ring-app-accent"
            placeholder="Ask about the codebase..."
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <button
            className="absolute right-1.5 top-1.5 rounded-md p-1.5 text-app-muted hover:bg-app-panelSoft hover:text-app-text"
            type="submit"
            disabled={isAsking}
            aria-label="Ask Localbrain"
          >
            <Send className="h-4 w-4" aria-hidden="true" />
          </button>
        </form>
        <div className="mt-2 flex flex-wrap gap-1.5">
          {suggestedQuestions.map((question) => (
            <button
              key={question}
              className="rounded-full border border-app-border bg-app-panelSoft px-2 py-1 text-[11px] text-app-muted hover:text-app-text"
              type="button"
              onClick={() => void askQuestion(question)}
            >
              {question}
            </button>
          ))}
        </div>
      </div>
      <div className="flex items-center justify-between border-t border-app-border px-3 py-2 text-[10px] text-app-muted">
        <span>Knowledge cached locally</span>
        <span className="flex items-center gap-1">
          <span className="h-1 w-1 rounded-full bg-emerald-500" />
          ready
        </span>
      </div>
    </aside>
  );
}

function InfoRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex justify-between gap-3">
      <span className="text-app-muted">{label}</span>
      <span className={`truncate text-right text-app-text ${mono ? 'font-mono' : ''}`}>
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
