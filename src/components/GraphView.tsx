import type { GraphViewData, GraphViewNode } from '../lib/graph';

interface GraphViewProps {
  data: GraphViewData | null;
  onSelectNode: (node: GraphViewNode) => void;
}

const fallbackNodes: GraphViewNode[] = [
  { id: 'feature-login', label: 'Localbrain', kind: 'file' },
  { id: 'parser', label: 'Parser', kind: 'component' },
  { id: 'graph', label: 'GraphStore', kind: 'component' },
  { id: 'search', label: 'Hybrid Search', kind: 'function' },
  { id: 'wiki', label: 'Wiki', kind: 'function' },
  { id: 'chat', label: 'ask_local', kind: 'function' },
  { id: 'api', label: 'Agent API', kind: 'import' },
  { id: 'metadata', label: 'SQLite', kind: 'class' },
];

const fallbackEdges = [
  ['feature-login', 'parser'],
  ['parser', 'graph'],
  ['graph', 'search'],
  ['search', 'chat'],
  ['graph', 'wiki'],
  ['chat', 'api'],
  ['search', 'metadata'],
];

export function GraphView({ data, onSelectNode }: GraphViewProps) {
  const nodes = data?.nodes.length ? data.nodes.slice(0, 12) : fallbackNodes;
  const edges = data?.edges.length
    ? data.edges.map((edge) => [edge.source, edge.target])
    : fallbackEdges;
  const width = 1180;
  const height = 720;
  const positioned = layoutNodes(nodes, width, height);

  return (
    <div className="absolute inset-0 bg-app-background">
      <div className="absolute left-3 top-3 z-10 flex items-center gap-2">
        <div className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px]">
          <span className="text-app-muted">Path:</span>{' '}
          <span className="font-medium text-app-text">{nodes[0]?.label ?? 'Localbrain'}</span>
        </div>
        <div className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px] text-app-muted">
          {nodes.length} nodes · {edges.length} edges
        </div>
      </div>

      <svg
        className="h-full w-full"
        viewBox={`0 0 ${width} ${height}`}
        role="img"
        aria-label="Knowledge graph"
      >
        <defs>
          <marker
            id="graph-arrow"
            viewBox="0 -5 10 10"
            refX="24"
            refY="0"
            markerWidth="6"
            markerHeight="6"
            orient="auto"
          >
            <path d="M0,-5L10,0L0,5" fill="rgb(var(--color-app-border))" />
          </marker>
        </defs>

        {edges.map(([sourceId, targetId], index) => {
          const source = positioned.get(sourceId);
          const target = positioned.get(targetId);
          if (!source || !target) {
            return null;
          }

          return (
            <line
              key={`${sourceId}-${targetId}-${index}`}
              x1={source.x}
              y1={source.y}
              x2={target.x}
              y2={target.y}
              stroke="rgb(var(--color-graph-edge))"
              strokeOpacity="0.75"
              strokeWidth="2"
              markerEnd="url(#graph-arrow)"
            />
          );
        })}

        {nodes.map((node) => {
          const position = positioned.get(node.id);
          if (!position) {
            return null;
          }
          const color = nodeColor(node.kind);

          return (
            <g
              key={node.id}
              className="cursor-pointer transition-opacity hover:opacity-90"
              transform={`translate(${position.x}, ${position.y})`}
              onClick={() => onSelectNode(node)}
            >
              <circle r="23" fill={color} fillOpacity="0.18" stroke={color} strokeWidth="1.7" />
              <circle r="8" fill={color} />
              <text
                y="38"
                textAnchor="middle"
                className="pointer-events-none fill-app-text text-[11px] font-semibold"
              >
                {shortLabel(node.label)}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function layoutNodes(nodes: GraphViewNode[], width: number, height: number) {
  const positions = new Map<string, { x: number; y: number }>();
  const startX = width * 0.36;
  const startY = height * 0.22;
  const stepX = 130;
  const stepY = 76;

  nodes.forEach((node, index) => {
    const bend = index % 3 === 0 ? -42 : index % 3 === 1 ? 0 : 42;
    positions.set(node.id, {
      x: startX + Math.min(index, 4) * stepX + (index > 4 ? (index - 4) * 30 : 0),
      y: startY + index * stepY + bend,
    });
  });

  return positions;
}

function nodeColor(kind: string) {
  if (kind === 'component') {
    return 'rgb(var(--color-graph-component))';
  }
  if (kind === 'import' || kind === 'export') {
    return 'rgb(var(--color-graph-api))';
  }
  if (kind === 'class' || kind === 'interface' || kind === 'type_alias') {
    return 'rgb(var(--color-graph-model))';
  }
  if (kind === 'method' || kind === 'function') {
    return 'rgb(var(--color-graph-service))';
  }

  return 'rgb(var(--color-graph-feature))';
}

function shortLabel(label: string) {
  return label.length > 18 ? `${label.slice(0, 15)}...` : label;
}
