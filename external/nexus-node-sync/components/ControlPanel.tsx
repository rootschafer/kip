import React from 'react';
import { Plus, FolderPlus, Globe, RefreshCw, HardDrive, Cpu, ShieldCheck, Trash2, Layers, Grid } from 'lucide-react';
import { FileNode, SyncRule } from '../types';

interface ControlPanelProps {
  onAddDevice: (type: 'local' | 'gdrive' | 'ssh') => void;
  onAutoSync: () => void;
  onAnalyze: () => void;
  onGroup: () => void;
  onDelete: () => void;
  selectedNodes: FileNode[];
  syncRules: SyncRule[];
  analysisResult: string | null;
  isAnalyzing: boolean;
}

const ControlPanel: React.FC<ControlPanelProps> = ({
  onAddDevice,
  onAutoSync,
  onAnalyze,
  onGroup,
  onDelete,
  selectedNodes,
  syncRules,
  analysisResult,
  isAnalyzing
}) => {
  const singleNode = selectedNodes.length === 1 ? selectedNodes[0] : null;
  const isMultiSelect = selectedNodes.length > 1;

  return (
    <div className="w-80 h-full glass-panel border-r-0 border-l border-white/10 flex flex-col shadow-2xl z-10 rounded-r-2xl m-2 my-2 rounded-l-none ml-0">
      
      {/* Header */}
      <div className="p-6 border-b border-white/5">
        <h2 className="text-xl font-bold text-white/90 flex items-center gap-3 tracking-tight">
          <div className="p-2 bg-blue-500/20 rounded-lg shadow-inner">
             <Cpu className="text-blue-400" size={20} />
          </div>
          Nexus
        </h2>
        <p className="text-[10px] text-slate-400 mt-1 uppercase tracking-widest pl-1">Universal Sync</p>
      </div>

      {/* Actions */}
      <div className="p-6 space-y-4">
        <h3 className="text-[10px] font-bold text-slate-400 uppercase tracking-widest">Add System</h3>
        <div className="grid grid-cols-3 gap-3">
          <button onClick={() => onAddDevice('local')} className="flex flex-col items-center justify-center p-3 bg-white/5 hover:bg-white/10 rounded-xl transition text-slate-200 border border-white/5 backdrop-blur-md group">
            <HardDrive size={20} className="mb-2 text-slate-400 group-hover:text-blue-400 transition-colors" />
            <span className="text-[10px] font-medium">Local</span>
          </button>
          <button onClick={() => onAddDevice('gdrive')} className="flex flex-col items-center justify-center p-3 bg-white/5 hover:bg-white/10 rounded-xl transition text-slate-200 border border-white/5 backdrop-blur-md group">
            <Globe size={20} className="mb-2 text-slate-400 group-hover:text-green-400 transition-colors" />
            <span className="text-[10px] font-medium">Cloud</span>
          </button>
          <button onClick={() => onAddDevice('ssh')} className="flex flex-col items-center justify-center p-3 bg-white/5 hover:bg-white/10 rounded-xl transition text-slate-200 border border-white/5 backdrop-blur-md group">
            <ShieldCheck size={20} className="mb-2 text-slate-400 group-hover:text-purple-400 transition-colors" />
            <span className="text-[10px] font-medium">SSH</span>
          </button>
        </div>
      </div>

      {/* Selection Info */}
      <div className="p-6 border-t border-white/5 flex-1 overflow-y-auto">
        {isMultiSelect ? (
            <div className="space-y-5">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 rounded-2xl flex items-center justify-center shadow-lg backdrop-blur-md border border-white/10 bg-indigo-500/20 text-indigo-300">
                    <Grid size={24}/>
                  </div>
                  <div className="overflow-hidden">
                    <h3 className="font-bold text-white text-md truncate">{selectedNodes.length} Items Selected</h3>
                    <span className="text-[10px] text-slate-400 px-2 py-0.5 bg-white/5 rounded-full border border-white/5 inline-block mt-1">
                      Bulk Action
                    </span>
                  </div>
                </div>
                
                <div className="grid grid-cols-2 gap-3">
                    <button onClick={onGroup} className="flex flex-col items-center justify-center p-4 bg-emerald-900/40 hover:bg-emerald-800/50 border border-emerald-500/20 rounded-xl text-emerald-200 transition">
                        <Layers size={20} className="mb-2" />
                        <span className="text-xs font-bold">Group</span>
                    </button>
                    <button onClick={onDelete} className="flex flex-col items-center justify-center p-4 bg-red-900/40 hover:bg-red-800/50 border border-red-500/20 rounded-xl text-red-200 transition">
                        <Trash2 size={20} className="mb-2" />
                        <span className="text-xs font-bold">Delete</span>
                    </button>
                </div>
                
                <div className="bg-white/5 rounded-xl p-3 max-h-40 overflow-y-auto">
                    <ul className="text-xs text-slate-400 space-y-1">
                        {selectedNodes.map(node => (
                            <li key={node.id} className="truncate">• {node.name}</li>
                        ))}
                    </ul>
                </div>
            </div>
        ) : singleNode ? (
          <div className="space-y-5">
            <div className="flex items-center gap-4">
              <div className={`w-12 h-12 rounded-2xl flex items-center justify-center shadow-lg backdrop-blur-md border border-white/10 ${singleNode.type === 'device' ? 'bg-blue-500/20 text-blue-300' : singleNode.type === 'group' ? 'bg-emerald-500/20 text-emerald-300' : 'bg-slate-700/30 text-slate-300'}`}>
                {singleNode.type === 'device' ? <HardDrive size={24}/> : singleNode.type === 'group' ? <Layers size={24}/> : <FolderPlus size={24}/>}
              </div>
              <div className="overflow-hidden">
                <h3 className="font-bold text-white text-md truncate">{singleNode.name}</h3>
                <span className="text-[10px] text-slate-400 px-2 py-0.5 bg-white/5 rounded-full border border-white/5 inline-block mt-1">
                  {singleNode.type}
                </span>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3 text-xs">
              <div className="p-3 bg-white/5 rounded-xl border border-white/5">
                <span className="block text-slate-500 mb-1 text-[10px] uppercase">Status</span>
                <span className={`font-bold flex items-center gap-1.5 ${singleNode.status === 'synced' ? 'text-emerald-400' : 'text-slate-300'}`}>
                  {singleNode.status === 'synced' && <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.8)]"></span>}
                  {singleNode.status.toUpperCase()}
                </span>
              </div>
              <div className="p-3 bg-white/5 rounded-xl border border-white/5">
                <span className="block text-slate-500 mb-1 text-[10px] uppercase">Size</span>
                <span className="font-mono text-slate-300">--</span>
              </div>
            </div>
            
            <button className="w-full py-2.5 bg-blue-600/80 hover:bg-blue-500/80 text-white rounded-xl text-xs font-semibold transition shadow-lg shadow-blue-900/30 border border-blue-400/20 backdrop-blur-sm">
                Open in Finder
            </button>
            
            <button onClick={onDelete} className="w-full py-2 bg-red-500/10 hover:bg-red-500/20 text-red-400 rounded-xl text-xs font-semibold transition border border-red-500/10">
                Delete Node
            </button>
          </div>
        ) : (
          <div className="h-32 flex flex-col items-center justify-center text-slate-500/50 border-2 border-dashed border-white/5 rounded-2xl">
            <span className="text-sm">Select items</span>
            <span className="text-[10px] mt-2">Shift + Drag to Lasso</span>
          </div>
        )}

        {/* Sync Rules List */}
        <div className="mt-8">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-[10px] font-bold text-slate-400 uppercase tracking-widest">Active Links</h3>
            <button onClick={onAutoSync} className="p-1.5 hover:bg-white/10 rounded-lg text-slate-400 hover:text-white transition">
               <RefreshCw size={12} />
            </button>
          </div>
          <div className="space-y-2">
            {syncRules.map(rule => (
              <div key={rule.id} className="p-3 bg-black/20 border border-white/5 rounded-xl flex items-center justify-between group hover:border-white/10 transition">
                <div className="flex items-center gap-3">
                    <div className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse shadow-[0_0_8px_rgba(52,211,153,0.6)]"></div>
                    <div className="flex flex-col">
                        <span className="text-[10px] text-slate-300 font-mono tracking-tight">
                            {rule.sourceId.substring(0,6)} <span className="text-slate-600">→</span> {rule.targetId.substring(0,6)}
                        </span>
                    </div>
                </div>
                <button className="text-slate-600 hover:text-red-400 opacity-0 group-hover:opacity-100 transition">×</button>
              </div>
            ))}
            {syncRules.length === 0 && (
                <p className="text-xs text-slate-600 italic">No active sync tasks.</p>
            )}
          </div>
        </div>

        {/* Gemini Analysis Section */}
        <div className="mt-6 pt-6 border-t border-white/5">
           <button 
             onClick={onAnalyze} 
             disabled={isAnalyzing}
             className="w-full flex items-center justify-center gap-2 py-3 bg-gradient-to-r from-purple-500/80 to-indigo-500/80 hover:from-purple-400/80 hover:to-indigo-400/80 text-white rounded-xl text-xs font-bold shadow-lg shadow-purple-900/20 transition disabled:opacity-50 backdrop-blur-sm border border-white/10">
               {isAnalyzing ? <RefreshCw className="animate-spin" size={14}/> : '✨'} 
               {isAnalyzing ? 'Analyzing Ecosystem...' : 'AI Optimization'}
           </button>
           
           {analysisResult && (
               <div className="mt-4 p-4 bg-indigo-950/40 border border-indigo-500/20 rounded-xl text-xs text-indigo-200 leading-relaxed max-h-40 overflow-y-auto backdrop-blur-md">
                   <pre className="whitespace-pre-wrap font-sans opacity-90">{analysisResult}</pre>
               </div>
           )}
        </div>
      </div>
    </div>
  );
};

export default ControlPanel;
