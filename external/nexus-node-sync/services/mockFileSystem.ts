import { FileNode, Link } from '../types';

const generateId = () => Math.random().toString(36).substr(2, 9);

export const initialNodes: FileNode[] = [
  { id: 'root_user', name: 'My Universe', type: 'root', status: 'idle', expanded: true },
  
  // Local Machine
  { id: 'dev_local', name: 'MacBook Pro', type: 'device', fsType: 'local', parentId: 'root_user', status: 'synced' },
  { id: 'folder_docs', name: 'Documents', type: 'folder', parentId: 'dev_local', status: 'synced' },
  { id: 'folder_pics', name: 'Pictures', type: 'folder', parentId: 'dev_local', status: 'idle' },
  
  // Google Drive
  { id: 'dev_gdrive', name: 'Google Drive', type: 'device', fsType: 'gdrive', parentId: 'root_user', status: 'synced' },
  { id: 'folder_g_work', name: 'Work Backups', type: 'folder', parentId: 'dev_gdrive', status: 'synced' },

  // SSH Server
  { id: 'dev_ssh', name: 'AWS EC2 (Ubuntu)', type: 'device', fsType: 'ssh', parentId: 'root_user', status: 'idle' },
  { id: 'folder_var', name: '/var/www', type: 'folder', parentId: 'dev_ssh', status: 'idle' },
];

export const initialLinks: Link[] = [
  { source: 'root_user', target: 'dev_local', type: 'hierarchy', id: 'l1' },
  { source: 'dev_local', target: 'folder_docs', type: 'hierarchy', id: 'l2' },
  { source: 'dev_local', target: 'folder_pics', type: 'hierarchy', id: 'l3' },
  
  { source: 'root_user', target: 'dev_gdrive', type: 'hierarchy', id: 'l4' },
  { source: 'dev_gdrive', target: 'folder_g_work', type: 'hierarchy', id: 'l5' },
  
  { source: 'root_user', target: 'dev_ssh', type: 'hierarchy', id: 'l6' },
  { source: 'dev_ssh', target: 'folder_var', type: 'hierarchy', id: 'l7' },

  // Initial Sync Link
  { source: 'folder_docs', target: 'folder_g_work', type: 'sync', id: 's1' },
];

export const generateChildren = (parentId: string, startX?: number, startY?: number, count: number = 3): { nodes: FileNode[], links: Link[] } => {
  const newNodes: FileNode[] = [];
  const newLinks: Link[] = [];

  for (let i = 0; i < count; i++) {
    const id = generateId();
    const isFolder = Math.random() > 0.5;
    const node: FileNode = {
      id,
      name: isFolder ? `Folder_${id.substr(0, 4)}` : `File_${id.substr(0, 4)}.dat`,
      type: isFolder ? 'folder' : 'file',
      parentId,
      status: 'idle',
      x: startX || 0, // Initialize at parent pos
      y: startY || 0
    };
    newNodes.push(node);
    newLinks.push({ source: parentId, target: id, type: 'hierarchy', id: `link_${id}` });
  }

  return { nodes: newNodes, links: newLinks };
};