import * as d3 from 'd3';
import { useEffect, useRef, useState } from 'react';
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
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  useEffect(() => {
    if (!svgRef.current || !containerRef.current || !data) {
      return;
    }

    const width = containerRef.current.clientWidth;
    const height = containerRef.current.clientHeight;

    const svgElement = svgRef.current;
    const svg = d3.select(svgElement);
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
    const neighborMap = new Map<string, Set<string>>();
    for (const link of links) {
      const source = String(link.source);
      const target = String(link.target);
      if (!neighborMap.has(source)) neighborMap.set(source, new Set());
      if (!neighborMap.has(target)) neighborMap.set(target, new Set());
      neighborMap.get(source)?.add(target);
      neighborMap.get(target)?.add(source);
    }
    const selectedNeighborhood =
      selectedNodeId == null
        ? null
        : new Set([selectedNodeId, ...(neighborMap.get(selectedNodeId) ?? new Set())]);

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
        setSelectedNodeId(d.id);
        onSelectNode(d);
      })
      .on('dblclick', (_event, d) => {
        d.fx = null;
        d.fy = null;
        simulation.alpha(0.4).restart();
      })
      .on('keydown', (event, d) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          setSelectedNodeId(d.id);
          onSelectNode(d);
        }
      });

    node
      .append('circle')
      .attr('r', (d) => (d.kind === 'file' ? 28 : 24))
      .attr('fill', (d) => d.color)
      .attr('fill-opacity', 0.15)
      .attr('stroke', (d) => d.color)
      .attr('stroke-width', (d) => {
        if (d.id === selectedNodeId) return 3.5;
        if (d.kind === 'file') return 2.5;
        return 1.5;
      })
      .attr('opacity', (d) => {
        if (!selectedNeighborhood) return 1;
        return selectedNeighborhood.has(d.id) ? 1 : 0.18;
      });

    node
      .append('circle')
      .attr('r', (d) => (d.kind === 'file' ? 10 : 8))
      .attr('fill', (d) => d.color)
      .attr('opacity', (d) => {
        if (!selectedNeighborhood) return 1;
        return selectedNeighborhood.has(d.id) ? 1 : 0.22;
      });

    node
      .append('text')
      .attr('y', (d) => (d.kind === 'file' ? 45 : 40))
      .attr('text-anchor', 'middle')
      .attr('font-size', '11px')
      .attr('font-weight', 'bold')
      .attr('fill', 'rgb(var(--color-app-text))')
      .attr('opacity', (d) => {
        if (!selectedNeighborhood) return 1;
        return selectedNeighborhood.has(d.id) ? 1 : 0.28;
      })
      .text((d) => {
        const label = d.kind === 'file' ? d.label.split('/').pop() || d.label : d.label;
        return label.length > 20 ? label.slice(0, 17) + '...' : label;
      });

    node
      .append('text')
      .attr('y', (d) => (d.kind === 'file' ? 56 : 51))
      .attr('text-anchor', 'middle')
      .attr('font-size', '9px')
      .attr('font-weight', '600')
      .attr('fill', 'rgb(var(--color-app-muted))')
      .attr('opacity', (d) => {
        if (!selectedNeighborhood) return 1;
        return selectedNeighborhood.has(d.id) ? 1 : 0.22;
      })
      .text((d) => d.kindLabel);

    link.attr('opacity', (d) => {
      if (!selectedNeighborhood) return 0.7;
      const sourceId = (d.source as D3Node).id;
      const targetId = (d.target as D3Node).id;
      return selectedNeighborhood.has(sourceId) && selectedNeighborhood.has(targetId) ? 0.9 : 0.08;
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
      d.fx = event.x;
      d.fy = event.y;
    }

    function fitToViewport() {
      const validNodes = nodes.filter((node) => Number.isFinite(node.x) && Number.isFinite(node.y));
      if (validNodes.length === 0) return;
      const xMin = d3.min(validNodes, (node) => node.x ?? 0) ?? 0;
      const xMax = d3.max(validNodes, (node) => node.x ?? 0) ?? width;
      const yMin = d3.min(validNodes, (node) => node.y ?? 0) ?? 0;
      const yMax = d3.max(validNodes, (node) => node.y ?? 0) ?? height;
      const graphWidth = Math.max(xMax - xMin, 1);
      const graphHeight = Math.max(yMax - yMin, 1);
      const scale = Math.min(width / (graphWidth + 120), height / (graphHeight + 120), 1.6);
      const centerX = (xMin + xMax) / 2;
      const centerY = (yMin + yMax) / 2;
      const transform = d3.zoomIdentity
        .translate(width / 2 - centerX * scale, height / 2 - centerY * scale)
        .scale(scale);
      svg.transition().duration(350).call(zoom.transform, transform);
    }

    function resetLayout() {
      for (const node of nodes) {
        node.fx = null;
        node.fy = null;
      }
      simulation.alpha(0.9).restart();
      setTimeout(fitToViewport, 300);
    }

    (
      svgElement as SVGSVGElement & { __graphReset?: () => void; __graphFit?: () => void }
    ).__graphReset = resetLayout;
    (
      svgElement as SVGSVGElement & { __graphReset?: () => void; __graphFit?: () => void }
    ).__graphFit = fitToViewport;

    setTimeout(fitToViewport, 250);

    return () => {
      const current = svgElement as SVGSVGElement & {
        __graphReset?: () => void;
        __graphFit?: () => void;
      };
      if (current) {
        current.__graphReset = undefined;
        current.__graphFit = undefined;
      }
      simulation.stop();
    };
  }, [data, onSelectNode, selectedNodeId]);

  return (
    <div ref={containerRef} className="absolute inset-0 bg-app-background overflow-hidden">
      <div className="absolute left-3 top-3 z-10 flex items-center gap-2 pointer-events-none">
        <div className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px]">
          <span className="text-app-muted">Nodes:</span>{' '}
          <span className="font-medium text-app-text">{data?.nodes.length ?? 0}</span>
        </div>
        <div className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px] text-app-muted">
          Drag pins node · Double-click unpins · Scroll to zoom
        </div>
      </div>
      <div className="absolute right-3 top-3 z-10 flex items-center gap-2">
        <button
          className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px] font-semibold text-app-muted hover:text-app-text"
          type="button"
          onClick={() => {
            const current = svgRef.current as SVGSVGElement & { __graphReset?: () => void };
            current?.__graphReset?.();
          }}
        >
          Reset Layout
        </button>
        <button
          className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px] font-semibold text-app-muted hover:text-app-text"
          type="button"
          onClick={() => {
            const current = svgRef.current as SVGSVGElement & { __graphFit?: () => void };
            current?.__graphFit?.();
          }}
        >
          Fit
        </button>
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
