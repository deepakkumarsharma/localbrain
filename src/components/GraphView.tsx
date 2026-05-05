import * as d3 from 'd3';
import { useEffect, useRef } from 'react';
import type { GraphViewData, GraphViewNode } from '../lib/graph';

interface GraphViewProps {
  data: GraphViewData | null;
  onSelectNode: (node: GraphViewNode) => void;
}

interface D3Node extends GraphViewNode, d3.SimulationNodeDatum {
  color: string;
}

interface D3Link extends d3.SimulationLinkDatum<D3Node> {
  source: string | D3Node;
  target: string | D3Node;
}

export function GraphView({ data, onSelectNode }: GraphViewProps) {
  const svgRef = useRef<SVGSVGElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!svgRef.current || !containerRef.current || !data) {
      return;
    }

    const width = containerRef.current.clientWidth;
    const height = containerRef.current.clientHeight;

    const svg = d3.select(svgRef.current);
    svg.selectAll('*').remove(); // Clear previous render

    const g = svg.append('g');

    // Zoom setup
    const zoom = d3
      .zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.1, 4])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      });

    svg.call(zoom);

    // Arrow marker
    svg
      .append('defs')
      .append('marker')
      .attr('id', 'arrow')
      .attr('viewBox', '0 -5 10 10')
      .attr('refX', 28)
      .attr('refY', 0)
      .attr('markerWidth', 6)
      .attr('markerHeight', 6)
      .attr('orient', 'auto')
      .append('path')
      .attr('d', 'M0,-5L10,0L0,5')
      .attr('fill', 'rgb(var(--color-app-border))');

    // Data preparation
    const nodes: D3Node[] = data.nodes.map((node) => ({
      ...node,
      color: nodeColor(node.kind),
    }));

    const links: D3Link[] = data.edges.map((edge) => ({
      source: edge.source,
      target: edge.target,
    }));

    // Simulation setup
    const simulation = d3
      .forceSimulation<D3Node>(nodes)
      .force(
        'link',
        d3
          .forceLink<D3Node, D3Link>(links)
          .id((d) => d.id)
          .distance(150)
          .strength(1),
      )
      .force('charge', d3.forceManyBody().strength(-800))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collide', d3.forceCollide(60));

    // Links rendering
    const link = g
      .append('g')
      .selectAll('line')
      .data(links)
      .join('line')
      .attr('stroke', 'rgb(var(--color-graph-edge))')
      .attr('stroke-opacity', 0.6)
      .attr('stroke-width', 1.5)
      .attr('marker-end', 'url(#arrow)');

    // Nodes rendering
    const node = g
      .append('g')
      .selectAll<SVGGElement, D3Node>('g')
      .data(nodes)
      .join('g')
      .attr(
        'class',
        'cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-app-accent rounded-full',
      )
      .attr('tabindex', 0)
      .attr('role', 'button')
      .attr('aria-label', (d) => d.label || 'graph node')
      .call(
        d3
          .drag<SVGGElement, D3Node>()
          .on('start', dragstarted)
          .on('drag', dragged)
          .on('end', dragended),
      )
      .on('click', (_event, d) => {
        onSelectNode(d);
      })
      .on('keydown', (event, d) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelectNode(d);
        }
      });

    node
      .append('circle')
      .attr('r', (d) => (d.kind === 'file' ? 28 : 24))
      .attr('fill', (d) => d.color)
      .attr('fill-opacity', 0.15)
      .attr('stroke', (d) => d.color)
      .attr('stroke-width', (d) => (d.kind === 'file' ? 2.5 : 1.5));

    node
      .append('circle')
      .attr('r', (d) => (d.kind === 'file' ? 10 : 8))
      .attr('fill', (d) => d.color);

    node
      .append('text')
      .attr('y', (d) => (d.kind === 'file' ? 45 : 40))
      .attr('text-anchor', 'middle')
      .attr('font-size', '11px')
      .attr('font-weight', 'bold')
      .attr('fill', 'rgb(var(--color-app-text))')
      .text((d) => {
        const label = d.kind === 'file' ? d.label.split('/').pop() || d.label : d.label;
        return label.length > 20 ? label.slice(0, 17) + '...' : label;
      });

    // Simulation tick
    simulation.on('tick', () => {
      link
        .attr('x1', (d) => (d.source as D3Node).x!)
        .attr('y1', (d) => (d.source as D3Node).y!)
        .attr('x2', (d) => (d.target as D3Node).x!)
        .attr('y2', (d) => (d.target as D3Node).y!);

      node.attr('transform', (d) => `translate(${d.x},${d.y})`);
    });

    function dragstarted(event: d3.D3DragEvent<SVGGElement, D3Node, D3Node>, d: D3Node) {
      if (!event.active) {
        simulation.alphaTarget(0.3).restart();
      }
      d.fx = d.x;
      d.fy = d.y;
    }

    function dragged(event: d3.D3DragEvent<SVGGElement, D3Node, D3Node>, d: D3Node) {
      d.fx = event.x;
      d.fy = event.y;
    }

    function dragended(event: d3.D3DragEvent<SVGGElement, D3Node, D3Node>, d: D3Node) {
      if (!event.active) {
        simulation.alphaTarget(0);
      }
      d.fx = null;
      d.fy = null;
    }

    return () => {
      simulation.stop();
    };
  }, [data, onSelectNode]);

  return (
    <div ref={containerRef} className="absolute inset-0 bg-app-background overflow-hidden">
      <div className="absolute left-3 top-3 z-10 flex items-center gap-2 pointer-events-none">
        <div className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px]">
          <span className="text-app-muted">Nodes:</span>{' '}
          <span className="font-medium text-app-text">{data?.nodes.length ?? 0}</span>
        </div>
        <div className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px] text-app-muted">
          Drag to explore · Scroll to zoom
        </div>
      </div>
      <svg ref={svgRef} className="h-full w-full" />
    </div>
  );
}

function nodeColor(kind: string) {
  if (kind === 'file') {
    return 'rgb(var(--color-app-text))';
  }
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
