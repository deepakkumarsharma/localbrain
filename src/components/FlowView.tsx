import { TrendingUp } from 'lucide-react';

const steps = [
  {
    title: 'Source selected',
    detail: 'Sidebar active file',
    color: 'rgb(var(--color-graph-feature))',
  },
  {
    title: 'Tree-sitter parses file',
    detail: 'parser/mod.rs',
    color: 'rgb(var(--color-graph-component))',
  },
  { title: 'Graph context loads', detail: 'graph/store.rs', color: 'rgb(var(--color-graph-api))' },
  {
    title: 'Search retrieves evidence',
    detail: 'search/mod.rs',
    color: 'rgb(var(--color-graph-service))',
  },
  { title: 'Local answer assembled', detail: 'llm/mod.rs', color: 'rgb(var(--color-graph-model))' },
];

export function FlowView() {
  return (
    <div className="absolute inset-0 overflow-auto bg-app-background">
      <div className="mx-auto max-w-[720px] px-8 py-12">
        <h2 className="mb-8 flex items-center gap-2 text-lg font-semibold">
          <TrendingUp className="h-5 w-5 text-app-accent" aria-hidden="true" />
          Localbrain Request Flow
        </h2>
        <div className="relative">
          <div className="absolute bottom-6 left-[27px] top-6 w-px bg-gradient-to-b from-app-border via-app-muted/25 to-app-border" />
          <div className="relative space-y-8">
            {steps.map((step, index) => (
              <div key={step.title} className="relative flex items-start gap-4">
                <div
                  className="relative z-10 flex h-[54px] w-[54px] items-center justify-center rounded-full border"
                  style={{
                    backgroundColor: 'rgb(var(--color-app-panel-soft))',
                    borderColor: step.color,
                  }}
                >
                  <div
                    className="flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold text-white"
                    style={{ backgroundColor: step.color }}
                  >
                    {index + 1}
                  </div>
                </div>
                <div className="pb-2 pt-1">
                  <div className="flex items-center gap-2">
                    <p className="text-sm font-semibold text-app-text">{step.title}</p>
                    <span className="rounded-md border border-app-border bg-app-panel px-1.5 py-0.5 text-[11px] text-app-muted">
                      local
                    </span>
                  </div>
                  <p className="mt-1 font-mono text-xs text-app-muted">{step.detail}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
