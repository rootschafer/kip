export type NodeStatus = 'synced' | 'syncing' | 'error' | 'offline' | 'idle';
export type NodeType = 'root' | 'device' | 'folder' | 'file' | 'group';
export type FileSystemType = 'local' | 'gdrive' | 'ssh' | 'usb';

export interface FileNode {
  id: string;
  name: string;
  type: NodeType;
  fsType?: FileSystemType; // Only for device nodes
  parentId?: string;
  size?: string;
  status: NodeStatus;
  childrenLoaded?: boolean; // If true, children are already in the graph
  expanded?: boolean;
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;
}

export interface Link {
  source: string | FileNode; // D3 converts string ID to object ref
  target: string | FileNode;
  type: 'hierarchy' | 'sync' | 'group';
  id: string; // Unique ID for the link
}

export interface SyncRule {
  id: string;
  sourceId: string;
  targetId: string;
  schedule: 'realtime' | 'hourly' | 'daily';
  status: 'active' | 'paused' | 'failed';
  lastSync?: Date;
}

export interface GeminiAnalysis {
  summary: string;
  suggestions: string[];
  riskLevel: 'low' | 'medium' | 'high';
}