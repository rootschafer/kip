import React, { useEffect, useRef, useState } from 'react';
import * as d3 from 'd3';
import { FileNode, Link } from '../types';

interface GraphCanvasProps {
  nodes: FileNode[];
  links: Link[];
  onNodeClick: (node: FileNode, multiSelect: boolean) => void;
  onNodeRightClick: (node: FileNode, event: React.MouseEvent) => void;
  onBackgroundClick: () => void;
  onSelectionBox: (extent: [[number, number], [number, number]] | null) => void;
  selectedNodeIds: Set<string>;
  linkModeActive: boolean;
}

const GraphCanvas: React.FC<GraphCanvasProps> = ({
  nodes,
  links,
  onNodeClick,
  onNodeRightClick,
  onBackgroundClick,
  onSelectionBox,
  selectedNodeIds,
  linkModeActive
}) => {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const simulationRef = useRef<d3.Simulation<FileNode, Link> | null>(null);
  const [selectionRect, setSelectionRect] = useState<{x: number, y: number, width: number, height: number} | null>(null);

  useEffect(() => {
    if (!svgRef.current || !containerRef.current) return;

    const width = containerRef.current.clientWidth;
    const height = containerRef.current.clientHeight;

    const svg = d3.select(svgRef.current)
      .attr("viewBox", [0, 0, width, height]);

    // --- Definitions (Gradients) ---
    const defs = svg.select<SVGDefsElement>("defs").empty() ? svg.append("defs") : svg.select<SVGDefsElement>("defs");
    defs.selectAll("*").remove();

    // Device Gradient (Blue)
    const deviceGrad = defs.append("radialGradient").attr("id", "grad-device").attr("cx", "30%").attr("cy", "30%").attr("r", "70%");
    deviceGrad.append("stop").attr("offset", "0%").attr("stop-color", "#60a5fa");
    deviceGrad.append("stop").attr("offset", "100%").attr("stop-color", "#1d4ed8");

    // Folder Gradient (Slate)
    const folderGrad = defs.append("radialGradient").attr("id", "grad-folder").attr("cx", "30%").attr("cy", "30%").attr("r", "70%");
    folderGrad.append("stop").attr("offset", "0%").attr("stop-color", "#94a3b8");
    folderGrad.append("stop").attr("offset", "100%").attr("stop-color", "#334155");

    // Group Gradient (Pleasant Green)
    const groupGrad = defs.append("radialGradient").attr("id", "grad-group").attr("cx", "30%").attr("cy", "30%").attr("r", "70%");
    groupGrad.append("stop").attr("offset", "0%").attr("stop-color", "#34d399"); // Emerald 400
    groupGrad.append("stop").attr("offset", "100%").attr("stop-color", "#059669"); // Emerald 600

    // Root Gradient (Purple)
    const rootGrad = defs.append("radialGradient").attr("id", "grad-root").attr("cx", "30%").attr("cy", "30%").attr("r", "70%");
    rootGrad.append("stop").attr("offset", "0%").attr("stop-color", "#c084fc");
    rootGrad.append("stop").attr("offset", "100%").attr("stop-color", "#7e22ce");

    // --- Container ---
    let g = svg.select<SVGGElement>(".graph-container");
    if (g.empty()) {
        g = svg.append("g").attr("class", "graph-container");
    }

    // --- Zoom & Lasso Behavior ---
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.1, 4])
      .on("zoom", (event) => {
        g.attr("transform", event.transform);
      })
      .filter((event) => {
        return !event.shiftKey && !event.button; 
      });

    svg.call(zoom);
    
    // Initial Center
    if (d3.zoomTransform(svg.node()!).k === 1 && d3.zoomTransform(svg.node()!).x === 0) {
       svg.call(zoom.transform, d3.zoomIdentity.translate(width / 2, height / 2).scale(1));
    }

    // --- Lasso Logic ---
    const dragBehavior = d3.drag<SVGSVGElement, unknown>()
        .filter(event => event.shiftKey)
        .on("start", (event) => {
            const [x, y] = d3.pointer(event, g.node());
            setSelectionRect({ x, y, width: 0, height: 0 });
        })
        .on("drag", (event) => {
            const [x, y] = d3.pointer(event, g.node());
            setSelectionRect(prev => {
                if(!prev) return null;
                return {
                    x: prev.x,
                    y: prev.y,
                    width: x - prev.x,
                    height: y - prev.y
                };
            });
        })
        .on("end", () => {
             if (selectionRect) {
                 const x = selectionRect.width > 0 ? selectionRect.x : selectionRect.x + selectionRect.width;
                 const y = selectionRect.height > 0 ? selectionRect.y : selectionRect.y + selectionRect.height;
                 const w = Math.abs(selectionRect.width);
                 const h = Math.abs(selectionRect.height);
                 
                 onSelectionBox([[x, y], [x + w, y + h]]);
             }
             setSelectionRect(null);
        });

    svg.call(dragBehavior);


    // --- Simulation ---
    let simulation = simulationRef.current;
    if (!simulation) {
        simulation = d3.forceSimulation<FileNode, Link>(nodes)
          .force("link", d3.forceLink<FileNode, Link>(links).id(d => d.id).distance(d => d.type === 'sync' ? 150 : d.type === 'group' ? 60 : 80))
          .force("charge", d3.forceManyBody().strength(-300))
          .force("x", d3.forceX().strength(0.04))
          .force("y", d3.forceY().strength(0.04))
          .force("collide", d3.forceCollide<FileNode>().radius(d => d.type === 'device' ? 45 : 30).iterations(2));
        simulationRef.current = simulation;
    } else {
        simulation.nodes(nodes);
        (simulation.force("link") as d3.ForceLink<FileNode, Link>).links(links);
        simulation.alpha(0.5).restart();
    }

    // --- Rendering ---
    let linkGroup = g.select<SVGGElement>(".links");
    if (linkGroup.empty()) linkGroup = g.append("g").attr("class", "links");
    let nodeGroup = g.select<SVGGElement>(".nodes");
    if (nodeGroup.empty()) nodeGroup = g.append("g").attr("class", "nodes");

    const linkSelection = linkGroup.selectAll<SVGLineElement, Link>("line")
      .data(links, d => d.id);

    const linkEnter = linkSelection.enter().append("line")
      .attr("stroke-width", d => d.type === 'sync' ? 1.5 : 1)
      .attr("stroke", d => d.type === 'sync' ? "#34d399" : "rgba(148, 163, 184, 0.2)")
      .attr("stroke-dasharray", d => d.type === 'sync' ? "4,4" : "0")
      .attr("class", d => d.type === 'sync' ? "animate-pulse" : "");

    const linkMerge = linkEnter.merge(linkSelection);
    linkSelection.exit().remove();

    const nodeSelection = nodeGroup.selectAll<SVGGElement, FileNode>("g")
      .data(nodes, d => d.id);

    const nodeEnter = nodeSelection.enter().append("g")
      .attr("cursor", "pointer")
      .call(d3.drag<SVGGElement, FileNode>()
        .on("start", dragstarted)
        .on("drag", dragged)
        .on("end", dragended));

    // Orb
    nodeEnter.append("circle")
      .attr("r", 0)
      .attr("fill", d => {
          if (d.type === 'root') return "url(#grad-root)";
          if (d.type === 'device') return "url(#grad-device)";
          if (d.type === 'group') return "url(#grad-group)";
          return "url(#grad-folder)";
      })
      .attr("stroke", "rgba(255,255,255,0.2)")
      .attr("stroke-width", 1)
      .transition().duration(500).ease(d3.easeBackOut)
      .attr("r", d => d.type === 'device' ? 24 : d.type === 'folder' || d.type === 'group' ? 18 : 8);

    // Glow for Selection
    nodeEnter.append("circle")
        .attr("class", "glow")
        .attr("r", d => d.type === 'device' ? 28 : 22)
        .attr("fill", "none")
        .attr("stroke", "white")
        .attr("stroke-opacity", 0)
        .attr("stroke-width", 2);

    // Label
    nodeEnter.append("text")
      .attr("dy", d => d.type === 'device' ? 40 : 34)
      .attr("text-anchor", "middle")
      .text(d => d.name)
      .attr("fill", "#e2e8f0")
      .attr("font-size", "10px")
      .attr("font-weight", "500")
      .attr("opacity", 0)
      .style("pointer-events", "none")
      .style("user-select", "none")
      .style("text-shadow", "0 2px 4px rgba(0,0,0,0.8)")
      .transition().delay(200).duration(300).attr("opacity", 1);

    const nodeMerge = nodeEnter.merge(nodeSelection);

    // Update Selection Glow
    nodeMerge.select(".glow")
        .attr("stroke-opacity", d => selectedNodeIds.has(d.id) ? 0.8 : 0)
        .attr("stroke", d => selectedNodeIds.has(d.id) ? "#60a5fa" : "white")
        .attr("stroke-dasharray", d => selectedNodeIds.has(d.id) && selectedNodeIds.size > 1 ? "3,2" : "0");

    nodeSelection.exit().transition().duration(300).attr("opacity", 0).remove();

    simulation.on("tick", () => {
      linkMerge
        .attr("x1", d => (d.source as FileNode).x!)
        .attr("y1", d => (d.source as FileNode).y!)
        .attr("x2", d => (d.target as FileNode).x!)
        .attr("y2", d => (d.target as FileNode).y!);

      nodeMerge.attr("transform", d => `translate(${d.x},${d.y})`);
    });

    // Handlers
    nodeEnter.on("click", (event, d) => {
      event.stopPropagation();
      onNodeClick(d, event.shiftKey || event.metaKey);
    });

    svg.on("click", (event) => {
       if((event.target.tagName === 'svg' || event.target.tagName === 'g') && !event.shiftKey) {
         onBackgroundClick();
       }
    });

    // --- Drag Logic ---
    function dragstarted(event: any, d: FileNode) {
      if (typeof d !== 'object') return; // Safety check
      if (!event.active) simulation?.alphaTarget(0.3).restart();
      
      if (!selectedNodeIds.has(d.id)) {
          onNodeClick(d, false);
      }

      d.fx = d.x;
      d.fy = d.y;
      
      if (selectedNodeIds.has(d.id)) {
          nodes.forEach(n => {
              if (selectedNodeIds.has(n.id) && n.id !== d.id) {
                  n.fx = n.x;
                  n.fy = n.y;
              }
          });
      }
    }

    function dragged(event: any, d: FileNode) {
      if (typeof d !== 'object') return; // Safety check
      const dx = event.x - (d.fx || 0);
      const dy = event.y - (d.fy || 0);

      d.fx = event.x;
      d.fy = event.y;

      if (selectedNodeIds.has(d.id)) {
          nodes.forEach(n => {
              if (selectedNodeIds.has(n.id) && n.id !== d.id) {
                  n.fx = (n.fx || n.x!) + dx;
                  n.fy = (n.fy || n.y!) + dy;
              }
          });
      }
    }

    function dragended(event: any, d: FileNode) {
      if (typeof d !== 'object') return; // Safety check
      if (!event.active) simulation?.alphaTarget(0);
      d.fx = null;
      d.fy = null;
      
      if (selectedNodeIds.has(d.id)) {
          nodes.forEach(n => {
              if (selectedNodeIds.has(n.id)) {
                  n.fx = null;
                  n.fy = null;
              }
          });
      }
    }

  }, [nodes, links, selectedNodeIds, onNodeClick, onBackgroundClick, onSelectionBox]);

  return (
    <div ref={containerRef} className={`w-full h-full overflow-hidden relative ${linkModeActive ? 'cursor-crosshair' : 'cursor-default'}`}>
      <svg ref={svgRef} className="w-full h-full block drop-shadow-2xl"></svg>
      {selectionRect && (
          <div style={{
              position: 'absolute',
              left: selectionRect.width > 0 ? selectionRect.x : selectionRect.x + selectionRect.width,
              top: selectionRect.height > 0 ? selectionRect.y : selectionRect.y + selectionRect.height,
              width: Math.abs(selectionRect.width),
              height: Math.abs(selectionRect.height),
              border: '1px solid rgba(96, 165, 250, 0.8)',
              backgroundColor: 'rgba(96, 165, 250, 0.2)',
              pointerEvents: 'none'
          }} />
      )}
    </div>
  );
};

export default GraphCanvas;