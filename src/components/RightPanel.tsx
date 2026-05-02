import { ExternalLink } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';

export function RightPanel() {
  const appVersion = useAppStore((state) => state.appVersion);

  return (
    <aside className="flex h-full flex-col border-l border-app-border bg-app-panel">
      <header className="border-b border-app-border px-4 py-4">
        <h2 className="text-lg font-semibold leading-tight">Details</h2>
        <p className="mt-1 text-xs font-medium text-app-muted">Version {appVersion}</p>
      </header>

      <section className="space-y-4 px-4 py-4">
        <div>
          <h3 className="text-xs font-medium uppercase text-app-muted">Citations</h3>
          <p className="mt-2 text-sm leading-6 text-app-muted">No citations yet.</p>
        </div>

        <div>
          <h3 className="text-xs font-medium uppercase text-app-muted">Actions</h3>
          <button
            className="mt-2 inline-flex h-8 items-center gap-2 rounded-md border border-app-border px-3 text-sm font-medium text-app-text hover:bg-app-panelSoft"
            type="button"
          >
            <ExternalLink className="h-4 w-4" aria-hidden="true" />
            Open in editor
          </button>
        </div>
      </section>
    </aside>
  );
}
