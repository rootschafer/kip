import React, { useState, useEffect, useCallback } from 'react';
import { initialNodes, initialLinks, generateChildren } from './services/mockFileSystem';
import { FileNode, Link, SyncRule } from './types';
import GraphCanvas from './components/GraphCanvas';
import ControlPanel from './components/ControlPanel';
import { analyzeFileSystem } from './services/geminiService';
import { Plus, Link as LinkIcon, AlertTriangle } from 'lucide-react';

const App: React.FC = () => {
  const [nodes, setNodes] = useState<FileNode[]>(initialNodes);
  const [links, setLinks] = useState<Link[]>(initialLinks);
  const [selectedNodeIds, setSelectedNodeIds] = useState<Set<string>>(new Set());
  const [linkMode, setLinkMode] = useState<{ active: boolean; sourceId: string | null }>({ active: false, sourceId: null });
  const [syncRules, setSyncRules] = useState<SyncRule[]>([
    { id: 'rule1', sourceId: 'folder_docs', targetId: 'folder_g_work', schedule: 'realtime', status: 'active' }
  ]);
  const [analysis, setAnalysis] = useState<string | null>(null);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [notification, setNotification] = useState<string | null>(null);

  // Helper: Resets D3 object references back to string IDs and strictly filters orphans.
  const sanitizeLinks = useCallback((currentLinks: Link[], currentNodes: FileNode[]) => {
      const nodeIds = new Set(currentNodes.map(n => n.id));
      return currentLinks
          .map(l => ({
              ...l,
              // Safely extract ID if D3 has already mutated it to an object
              source: typeof l.source === 'object' ? (l.source as any).id : l.source,
              target: typeof l.target === 'object' ? (l.target as any).id : l.target
          }))
          .filter(l => nodeIds.has(l.source as string) && nodeIds.has(l.target as string));
  }, []);

  // --- Handlers ---

  const handleNodeClick = useCallback((node: FileNode, multiSelect: boolean) => {
    // Link Mode Interaction (Single select mostly)
    if (linkMode.active) {
      if (linkMode.sourceId === null) {
        setLinkMode({ ...linkMode, sourceId: node.id });
        setNotification(`Select target node to sync with ${node.name}`);
      } else if (linkMode.sourceId !== node.id) {
        // Create Link
        const newLink: Link = {
          source: linkMode.sourceId,
          target: node.id,
          type: 'sync',
          id: `sync_${Date.now()}`
        };
        const newRule: SyncRule = {
          id: `rule_${Date.now()}`,
          sourceId: linkMode.sourceId,
          targetId: node.id,
          schedule: 'realtime',
          status: 'active'
        };
        
        // Update state atomically
        const nextLinks = sanitizeLinks([...links, newLink], nodes);
        setLinks(nextLinks);
        setSyncRules(prev => [...prev, newRule]);
        setLinkMode({ active: false, sourceId: null });
        setNotification(`Sync established successfully!`);
        setTimeout(() => setNotification(null), 3000);
      }
      return;
    }

    // Selection Logic
    if (multiSelect) {
        setSelectedNodeIds(prev => {
            const next = new Set(prev);
            if (next.has(node.id)) next.delete(node.id);
            else next.add(node.id);
            return next;
        });
    } else {
        setSelectedNodeIds(prev => {
            if (prev.has(node.id) && prev.size === 1) return prev;
            return new Set([node.id]);
        });
    }

    // Expand logic (Only if single node click and not loaded)
    if ((node.type === 'device' || node.type === 'folder' || node.type === 'group') && !node.childrenLoaded && !multiSelect) {
      const currentNodeState = nodes.find(n => n.id === node.id);
      const startX = currentNodeState?.x;
      const startY = currentNodeState?.y;

      const { nodes: newNodes, links: newLinks } = generateChildren(node.id, startX, startY);
      
      const updatedNodes = nodes.map(n => n.id === node.id ? { ...n, childrenLoaded: true, expanded: true } : n);
      const finalNodes = [...updatedNodes, ...newNodes];
      const finalLinks = sanitizeLinks([...links, ...newLinks], finalNodes);
      
      setNodes(finalNodes);
      setLinks(finalLinks);
    }
  }, [linkMode, nodes, links, sanitizeLinks]);

  const handleNodeRightClick = useCallback((node: FileNode, event: React.MouseEvent) => {
     setSelectedNodeIds(new Set([node.id]));
  }, []);

  const handleBackgroundClick = useCallback(() => {
    setSelectedNodeIds(new Set());
    if (linkMode.active) {
       setLinkMode({ active: false, sourceId: null });
       setNotification(null);
    }
  }, [linkMode]);

  const handleSelectionBox = useCallback((extent: [[number, number], [number, number]] | null) => {
      if (!extent) return;
      const [[x0, y0], [x1, y1]] = extent;
      
      const newSelection = new Set<string>();
      nodes.forEach(node => {
          if (node.x && node.y && node.x >= x0 && node.x <= x1 && node.y >= y0 && node.y <= y1) {
              newSelection.add(node.id);
          }
      });
      
      if (newSelection.size > 0) {
          setSelectedNodeIds(newSelection);
      }
  }, [nodes]);

  const handleAddDevice = (type: 'local' | 'gdrive' | 'ssh') => {
    const newId = `dev_${Date.now()}`;
    const newNode: FileNode = {
      id: newId,
      name: type === 'local' ? 'New Local Drive' : type === 'gdrive' ? 'New Google Drive' : 'New SSH Connection',
      type: 'device',
      fsType: type,
      parentId: 'root_user',
      status: 'idle'
    };
    const newLink: Link = {
      source: 'root_user',
      target: newId,
      type: 'hierarchy',
      id: `l_${newId}`
    };

    const nextNodes = [...nodes, newNode];
    const nextLinks = sanitizeLinks([...links, newLink], nextNodes);

    setNodes(nextNodes);
    setLinks(nextLinks);
    setNotification(`Added new ${type} device`);
    setTimeout(() => setNotification(null), 2000);
  };

  const handleGroupNodes = () => {
      if (selectedNodeIds.size < 2) return;
      
      const selected = nodes.filter(n => selectedNodeIds.has(n.id));
      const groupId = `group_${Date.now()}`;
      
      // Default parent to root if logic fails
      const commonParentId = selected[0].parentId || 'root_user';
      
      const avgX = selected.reduce((sum, n) => sum + (n.x || 0), 0) / selected.length;
      const avgY = selected.reduce((sum, n) => sum + (n.y || 0), 0) / selected.length;

      const groupNode: FileNode = {
          id: groupId,
          name: 'New Group',
          type: 'group',
          parentId: commonParentId,
          status: 'idle',
          childrenLoaded: true,
          expanded: true,
          x: avgX,
          y: avgY
      };

      // 1. Update nodes
      const updatedNodes = nodes.map(n => {
          if (selectedNodeIds.has(n.id)) {
              return { ...n, parentId: groupId };
          }
          return n;
      });

      // 2. Filter old hierarchy links pointing TO the selected nodes
      const linksToKeep = links.filter(l => {
          const targetId = typeof l.target === 'object' ? (l.target as FileNode).id : l.target;
          // If target is selected, we remove the link (because we are moving it under a group)
          // Exception: If source is also selected, it's an internal link, keep it? 
          // Current logic: Hierarchy is flat mostly. 
          return !selectedNodeIds.has(targetId as string);
      });

      const linkToGroup: Link = {
          source: commonParentId,
          target: groupId,
          type: 'hierarchy',
          id: `l_${groupId}`
      };

      const newGroupLinks: Link[] = selected.map(n => ({
          source: groupId,
          target: n.id,
          type: 'group', // Changed to 'group'
          id: `l_${groupId}_${n.id}`
      }));

      const finalNodes = [...updatedNodes, groupNode];
      const finalLinks = sanitizeLinks([...linksToKeep, linkToGroup, ...newGroupLinks], finalNodes);

      setNodes(finalNodes);
      setLinks(finalLinks);
      setSelectedNodeIds(new Set([groupId]));
      setNotification("Group created");
  };

  const handleDeleteNodes = () => {
      if (selectedNodeIds.size === 0) return;
      
      if (window.confirm(`Are you sure you want to delete ${selectedNodeIds.size} items?`)) {
          const remainingNodes = nodes.filter(n => !selectedNodeIds.has(n.id));
          
          // Strict filtering of links is handled by sanitizeLinks, but we can pre-filter for efficiency
          const remainingLinks = links.filter(l => {
              const sid = typeof l.source === 'object' ? (l.source as any).id : l.source;
              const tid = typeof l.target === 'object' ? (l.target as any).id : l.target;
              return !selectedNodeIds.has(sid) && !selectedNodeIds.has(tid);
          });
          
          setNodes(remainingNodes);
          setLinks(sanitizeLinks(remainingLinks, remainingNodes));
          setSelectedNodeIds(new Set());
          setNotification("Items deleted");
      }
  };

  const handleAnalyze = async () => {
    setIsAnalyzing(true);
    const result = await analyzeFileSystem(nodes, syncRules);
    setAnalysis(result);
    setIsAnalyzing(false);
  };

  const toggleLinkMode = () => {
    setLinkMode(prev => {
        const nextState = !prev.active;
        if(nextState) setNotification("Select a SOURCE node for synchronization.");
        else setNotification(null);
        return { active: nextState, sourceId: null };
    });
  };

  return (
    <div className="flex w-full h-screen text-white overflow-hidden font-sans">
      
      {/* Sidebar */}
      <ControlPanel 
        onAddDevice={handleAddDevice}
        onAutoSync={() => setNotification("All sync tasks triggered.")}
        onAnalyze={handleAnalyze}
        onGroup={handleGroupNodes}
        onDelete={handleDeleteNodes}
        selectedNodes={nodes.filter(n => selectedNodeIds.has(n.id))}
        syncRules={syncRules}
        analysisResult={analysis}
        isAnalyzing={isAnalyzing}
      />

      {/* Main Canvas Area */}
      <div className="flex-1 relative">
        
        {/* Top Bar / Toolbar */}
        <div className="absolute top-4 left-4 right-4 z-10 flex items-center justify-between pointer-events-none">
          <div className="pointer-events-auto glass-panel rounded-2xl p-2 flex gap-2 shadow-2xl">
             <button 
                onClick={toggleLinkMode}
                className={`px-4 py-2 rounded-xl flex items-center gap-2 text-xs font-bold transition shadow-lg ${linkMode.active ? 'bg-emerald-500/80 text-white shadow-emerald-500/30' : 'bg-white/5 text-slate-300 hover:bg-white/10 hover:text-white'}`}
             >
                <LinkIcon size={16} />
                {linkMode.active ? 'CANCEL LINK' : 'ADD SYNC'}
             </button>
             
             <div className="w-px bg-white/10 mx-1"></div>

             <div className="px-3 flex items-center text-xs text-slate-300 font-mono gap-4">
                <span>NODES: <b className="text-white">{nodes.length}</b></span>
                <span>SYNCS: <b className="text-white">{syncRules.length}</b></span>
             </div>
          </div>
        </div>

        {/* Notification Toast */}
        {notification && (
          <div className="absolute top-24 left-1/2 -translate-x-1/2 z-50 glass-panel border border-white/20 text-white px-6 py-3 rounded-full shadow-2xl flex items-center gap-3 animate-bounce-in backdrop-blur-xl">
             <div className="w-2 h-2 rounded-full bg-blue-400 shadow-[0_0_10px_rgba(96,165,250,0.8)]"></div>
             <span className="text-xs font-medium tracking-wide">{notification}</span>
          </div>
        )}

        {/* Graph Visualizer */}
        <GraphCanvas 
          nodes={nodes}
          links={links}
          onNodeClick={handleNodeClick}
          onNodeRightClick={handleNodeRightClick}
          onBackgroundClick={handleBackgroundClick}
          onSelectionBox={handleSelectionBox}
          selectedNodeIds={selectedNodeIds}
          linkModeActive={linkMode.active}
        />
        
        {/* Helper overlay for link mode */}
        {linkMode.active && (
            <div className="absolute bottom-10 left-1/2 -translate-x-1/2 pointer-events-none">
               <div className="glass-panel text-emerald-300 px-8 py-3 rounded-2xl border border-emerald-500/30 text-sm font-bold shadow-2xl backdrop-blur-md">
                   {linkMode.sourceId ? 'Select TARGET node to complete sync...' : 'Select SOURCE node to start sync...'}
               </div>
            </div>
        )}
      </div>
    </div>
  );
};

export default App;