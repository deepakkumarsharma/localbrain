import * as d3 from 'd3';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { GraphViewData, GraphViewNode } from '../lib/graph';
import { useAppStore } from '../store/useAppStore';

interface GraphViewProps {
  data: GraphViewData | null;
  onSelectNode: (node: GraphViewNode) => void;
}

type GraphMode = 'structure' | 'code';

interface RenderNode extends d3.SimulationNodeDatum {
  id: string;
  label: string;
  kind: string;
  color: string;
  rawId?: string;
  path?: string;
  isFolder?: boolean;
  isFile?: boolean;
}

interface RenderLink extends d3.SimulationLinkDatum<RenderNode> {
  id: string;
  source: string | RenderNode;
  target: string | RenderNode;
  label: string;
}

interface StructureGraph {
  nodes: RenderNode[];
  edges: RenderLink[];
}

const ROOT_FOLDER = '';

export function GraphView({ data, onSelectNode }: GraphViewProps) {
  const svgRef = useRef<SVGSVGElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const nodeLayoutRef = useRef<
    Map<string, { x: number; y: number; fx: number | null; fy: number | null }>
  >(new Map());
  const zoomTransformRef = useRef(d3.zoomIdentity);
  const shouldAutoFitRef = useRef(true);
  const [mode, setMode] = useState<GraphMode>('structure');
  const [showEdgeLabels, setShowEdgeLabels] = useState(false);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set([ROOT_FOLDER]));

  const { projectPath, indexPathSummary, activeSourcePath, setActiveSourcePath } = useAppStore();

  useEffect(() => {
    setMode('structure');
    setExpandedFolders(new Set([ROOT_FOLDER]));
    shouldAutoFitRef.current = true;
    nodeLayoutRef.current.clear();
    zoomTransformRef.current = d3.zoomIdentity;
  }, [projectPath]);

  useEffect(() => {
    if (!activeSourcePath) return;
    setExpandedFolders((current) => {
      const next = new Set(current);
      next.add(ROOT_FOLDER);
      for (const folder of folderAncestors(activeSourcePath)) {
        next.add(folder);
      }
      return next;
    });
  }, [activeSourcePath]);

  const structureGraph = useMemo(() => {
    if (!projectPath) {
      return null;
    }
    const files = indexPathSummary?.files?.map((file) => file.path) ?? [];
    const projectName = projectPath.split(/[\\/]/).filter(Boolean).pop() || 'project';
    return buildStructureGraph(projectName, files, expandedFolders);
  }, [expandedFolders, indexPathSummary, projectPath]);

  const renderData = useMemo(() => {
    if (mode === 'structure') {
      return structureGraph;
    }
    if (!data) {
      return null;
    }

    const nodes: RenderNode[] = data.nodes.map((node) => ({
      id: node.id,
      rawId: node.id,
      label: node.kind === 'file' ? fileName(node.label) : node.label,
      kind: node.kind,
      color: nodeColor(node.kind),
    }));
    const edges: RenderLink[] = data.edges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      label: edge.label,
    }));

    return {
      nodes,
      edges,
    };
  }, [data, mode, structureGraph]);

  useEffect(() => {
    if (!svgRef.current || !containerRef.current || !renderData) {
      return;
    }

    const width = containerRef.current.clientWidth;
    const height = containerRef.current.clientHeight;

    const svgElement = svgRef.current;
    const svg = d3.select(svgElement);
    svg.selectAll('*').remove();

    const g = svg.append('g');

    const zoom = d3
      .zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.1, 4])
      .on('zoom', (event) => {
        zoomTransformRef.current = event.transform;
        g.attr('transform', event.transform);
      });

    svg.call(zoom);
    svg.call(zoom.transform, zoomTransformRef.current);

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

    const nodes = renderData.nodes.map((node) => ({ ...node }));
    const links = renderData.edges.map((edge) => ({ ...edge }));
    for (const current of nodes) {
      const existing = nodeLayoutRef.current.get(current.id);
      if (!existing) continue;
      current.x = existing.x;
      current.y = existing.y;
      current.fx = existing.fx;
      current.fy = existing.fy;
    }

    const simulation = d3
      .forceSimulation<RenderNode>(nodes)
      .force(
        'link',
        d3
          .forceLink<RenderNode, RenderLink>(links)
          .id((d) => d.id)
          .distance(mode === 'structure' ? 140 : 155)
          .strength(0.95),
      )
      .force('charge', d3.forceManyBody().strength(mode === 'structure' ? -1000 : -860))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collide', d3.forceCollide(mode === 'structure' ? 66 : 62));

    const link = g
      .append('g')
      .selectAll('line')
      .data(links)
      .join('line')
      .attr('stroke', 'rgb(var(--color-graph-edge))')
      .attr('stroke-opacity', 0.64)
      .attr('stroke-width', 1.2)
      .attr('marker-end', 'url(#arrow)');

    const linkLabels = g
      .append('g')
      .selectAll('text')
      .data(links)
      .join('text')
      .attr('font-size', '10px')
      .attr('font-weight', '700')
      .attr('fill', 'rgb(var(--color-app-muted))')
      .attr('text-anchor', 'middle')
      .attr('pointer-events', 'none')
      .style('display', showEdgeLabels ? 'block' : 'none')
      .text((d) => readableEdgeLabel(d.label));

    const node = g
      .append('g')
      .selectAll<SVGGElement, RenderNode>('g')
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
          .drag<SVGGElement, RenderNode>()
          .on('start', dragstarted)
          .on('drag', dragged)
          .on('end', dragended),
      )
      .on('click', (_event, d) => {
        if (mode === 'structure') {
          d.fx = d.x ?? null;
          d.fy = d.y ?? null;
        }

        if (mode === 'structure') {
          if (d.isFolder) {
            const folderPath = d.path ?? ROOT_FOLDER;
            setExpandedFolders((current) => {
              const next = new Set(current);
              if (next.has(folderPath)) {
                next.delete(folderPath);
              } else {
                next.add(folderPath);
              }
              next.add(ROOT_FOLDER);
              return next;
            });
            return;
          }

          if (d.isFile && d.path) {
            setActiveSourcePath(d.path);
            setMode('code');
            onSelectNode({ id: d.path, label: d.path, kind: 'file' });
            return;
          }
        }

        onSelectNode({ id: d.rawId ?? d.id, label: d.label, kind: d.kind });
      })
      .on('dblclick', (_event, d) => {
        d.fx = null;
        d.fy = null;
        simulation.alpha(0.4).restart();
      })
      .on('keydown', (event, d) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelectNode({ id: d.rawId ?? d.id, label: d.label, kind: d.kind });
        }
      });

    node
      .append('circle')
      .attr('r', (d) => {
        if (mode === 'structure' && d.isFolder) return 30;
        if (d.kind === 'file') return 26;
        return 22;
      })
      .attr('fill', (d) => d.color)
      .attr('fill-opacity', 0.13)
      .attr('stroke', (d) => d.color)
      .attr('stroke-width', (d) => (d.kind === 'file' ? 2.2 : 1.4))
      .attr('opacity', 1);

    node
      .append('circle')
      .attr('r', (d) => (d.kind === 'file' ? 9 : 7))
      .attr('fill', (d) => d.color)
      .attr('opacity', 0.95);

    node
      .append('text')
      .attr('y', (d) => (d.kind === 'file' ? 42 : 38))
      .attr('text-anchor', 'middle')
      .attr('font-size', '10px')
      .attr('font-weight', '700')
      .attr('fill', 'rgb(var(--color-app-text))')
      .attr('opacity', 0.88)
      .text((d) => {
        const label = d.label;
        return label.length > 18 ? `${label.slice(0, 15)}...` : label;
      });

    link.attr('opacity', 0.72);

    linkLabels.attr('opacity', 0.75);

    simulation.on('tick', () => {
      link
        .attr('x1', (d) => (d.source as RenderNode).x!)
        .attr('y1', (d) => (d.source as RenderNode).y!)
        .attr('x2', (d) => (d.target as RenderNode).x!)
        .attr('y2', (d) => (d.target as RenderNode).y!);

      linkLabels
        .attr('x', (d) => ((d.source as RenderNode).x! + (d.target as RenderNode).x!) / 2)
        .attr('y', (d) => ((d.source as RenderNode).y! + (d.target as RenderNode).y!) / 2 - 8);

      node.attr('transform', (d) => `translate(${d.x},${d.y})`);
      nodeLayoutRef.current.clear();
      for (const current of nodes) {
        if (!Number.isFinite(current.x) || !Number.isFinite(current.y)) continue;
        nodeLayoutRef.current.set(current.id, {
          x: current.x ?? 0,
          y: current.y ?? 0,
          fx: current.fx ?? null,
          fy: current.fy ?? null,
        });
      }
    });

    function dragstarted(
      event: d3.D3DragEvent<SVGGElement, RenderNode, RenderNode>,
      d: RenderNode,
    ) {
      if (!event.active) {
        simulation.alphaTarget(0.28).restart();
      }
      d.fx = d.x;
      d.fy = d.y;
    }

    function dragged(event: d3.D3DragEvent<SVGGElement, RenderNode, RenderNode>, d: RenderNode) {
      d.fx = event.x;
      d.fy = event.y;
    }

    function dragended(event: d3.D3DragEvent<SVGGElement, RenderNode, RenderNode>, d: RenderNode) {
      if (!event.active) {
        simulation.alphaTarget(0);
      }
      d.fx = event.x;
      d.fy = event.y;
    }

    function fitToViewport() {
      const validNodes = nodes.filter(
        (current) => Number.isFinite(current.x) && Number.isFinite(current.y),
      );
      if (validNodes.length === 0) return;
      const xMin = d3.min(validNodes, (current) => current.x ?? 0) ?? 0;
      const xMax = d3.max(validNodes, (current) => current.x ?? 0) ?? width;
      const yMin = d3.min(validNodes, (current) => current.y ?? 0) ?? 0;
      const yMax = d3.max(validNodes, (current) => current.y ?? 0) ?? height;
      const graphWidth = Math.max(xMax - xMin, 1);
      const graphHeight = Math.max(yMax - yMin, 1);
      const scale = Math.min(width / (graphWidth + 140), height / (graphHeight + 140), 1.45);
      const centerX = (xMin + xMax) / 2;
      const centerY = (yMin + yMax) / 2;
      const transform = d3.zoomIdentity
        .translate(width / 2 - centerX * scale, height / 2 - centerY * scale)
        .scale(scale);
      svg.transition().duration(320).call(zoom.transform, transform);
    }

    function resetLayout() {
      for (const current of nodes) {
        current.fx = null;
        current.fy = null;
      }
      simulation.alpha(0.95).restart();
      shouldAutoFitRef.current = true;
      setTimeout(fitToViewport, 240);
    }

    (
      svgElement as SVGSVGElement & { __graphReset?: () => void; __graphFit?: () => void }
    ).__graphReset = resetLayout;
    (
      svgElement as SVGSVGElement & { __graphReset?: () => void; __graphFit?: () => void }
    ).__graphFit = fitToViewport;

    if (shouldAutoFitRef.current) {
      shouldAutoFitRef.current = false;
      setTimeout(fitToViewport, 220);
    }

    return () => {
      const current = svgElement as SVGSVGElement & {
        __graphReset?: () => void;
        __graphFit?: () => void;
      };
      current.__graphReset = undefined;
      current.__graphFit = undefined;
      simulation.stop();
    };
  }, [expandedFolders, mode, onSelectNode, renderData, setActiveSourcePath, showEdgeLabels]);

  return (
    <div ref={containerRef} className="absolute inset-0 bg-app-background overflow-hidden">
      <div className="absolute left-3 top-3 z-10 flex items-center gap-2 pointer-events-none">
        <div className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px]">
          <span className="text-app-muted">Nodes:</span>{' '}
          <span className="font-medium text-app-text">{renderData?.nodes.length ?? 0}</span>
        </div>
        <div className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px] text-app-muted">
          {mode === 'structure'
            ? 'Click folders to expand · Click file to open code map'
            : 'Drag nodes · Double-click to unpin · Scroll to zoom'}
        </div>
      </div>
      <div className="absolute right-3 top-3 z-10 flex items-center gap-2">
        <button
          className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px] font-semibold text-app-muted hover:text-app-text"
          type="button"
          onClick={() => setMode((current) => (current === 'structure' ? 'code' : 'structure'))}
        >
          {mode === 'structure' ? 'Switch to Code Map' : 'Switch to Project Map'}
        </button>
        <button
          className="rounded-lg border border-app-border bg-app-panel/90 px-2.5 py-1.5 text-[11px] font-semibold text-app-muted hover:text-app-text"
          type="button"
          onClick={() => setShowEdgeLabels((current) => !current)}
        >
          {showEdgeLabels ? 'Labels On' : 'Labels Off'}
        </button>
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

function readableEdgeLabel(label: string) {
  const normalized = label.trim().toLowerCase();
  if (normalized === 'contains') return 'contains';
  if (normalized === 'imports') return 'imports';
  if (normalized === 'calls') return 'calls';
  if (normalized === 'defines') return 'defines';
  if (normalized === 'references') return 'references';
  return normalized || 'related';
}

function buildStructureGraph(
  projectName: string,
  paths: string[],
  expandedFolders: Set<string>,
): StructureGraph {
  const root: FolderNode = { name: projectName, path: ROOT_FOLDER, folders: new Map(), files: [] };

  for (const path of paths) {
    const parts = path.split('/').filter(Boolean);
    let cursor = root;
    let currentPath = '';

    for (let i = 0; i < parts.length; i += 1) {
      const part = parts[i];
      const isFile = i === parts.length - 1;

      if (isFile) {
        cursor.files.push({ name: part, path });
        continue;
      }

      currentPath = currentPath ? `${currentPath}/${part}` : part;
      if (!cursor.folders.has(part)) {
        cursor.folders.set(part, {
          name: part,
          path: currentPath,
          folders: new Map(),
          files: [],
        });
      }
      cursor = cursor.folders.get(part)!;
    }
  }

  const nodes: RenderNode[] = [];
  const edges: RenderLink[] = [];

  const rootId = structureFolderId(ROOT_FOLDER);
  nodes.push({
    id: rootId,
    label: projectName,
    kind: 'project',
    color: 'rgb(var(--color-graph-feature))',
    isFolder: true,
    path: ROOT_FOLDER,
  });

  const walk = (folder: FolderNode, parentId: string) => {
    const childFolders = Array.from(folder.folders.values()).sort((a, b) =>
      a.name.localeCompare(b.name),
    );
    const childFiles = folder.files.sort((a, b) => a.name.localeCompare(b.name));

    for (const childFolder of childFolders) {
      const folderId = structureFolderId(childFolder.path);
      nodes.push({
        id: folderId,
        label: childFolder.name,
        kind: 'folder',
        color: 'rgb(var(--color-graph-component))',
        isFolder: true,
        path: childFolder.path,
      });
      edges.push({
        id: `${parentId}->${folderId}`,
        source: parentId,
        target: folderId,
        label: 'contains',
      });

      if (expandedFolders.has(childFolder.path)) {
        walk(childFolder, folderId);
      }
    }

    for (const childFile of childFiles) {
      const fileId = structureFileId(childFile.path);
      nodes.push({
        id: fileId,
        label: childFile.name,
        kind: 'file',
        color: 'rgb(var(--color-app-text))',
        isFile: true,
        path: childFile.path,
      });
      edges.push({
        id: `${parentId}->${fileId}`,
        source: parentId,
        target: fileId,
        label: 'contains',
      });
    }
  };

  walk(root, rootId);
  return { nodes, edges };
}

interface FolderNode {
  name: string;
  path: string;
  folders: Map<string, FolderNode>;
  files: Array<{ name: string; path: string }>;
}

function structureFolderId(path: string) {
  return `folder:${path || '__root__'}`;
}

function structureFileId(path: string) {
  return `file:${path}`;
}

function folderAncestors(path: string): string[] {
  const parts = path.split('/').filter(Boolean);
  const folders: string[] = [];
  let cursor = '';
  for (let i = 0; i < Math.max(0, parts.length - 1); i += 1) {
    cursor = cursor ? `${cursor}/${parts[i]}` : parts[i];
    folders.push(cursor);
  }
  return folders;
}

function fileName(path: string) {
  const parts = path.split('/');
  return parts[parts.length - 1] || path;
}

function nodeColor(kind: string) {
  if (kind === 'file') {
    return 'rgb(var(--color-app-text))';
  }
  if (kind === 'component' || kind === 'folder') {
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
