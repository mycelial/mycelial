import { configurePipes } from './pipes.js';
import { getClients } from './clients.js';
import { Client, DataNode } from '../types.js';
import { getWorkspace } from './workspaces.ts';

type DataLoaderParams = { params: { workspaceId: string } };

export default async function dataLoader({ params }: DataLoaderParams) {
  const [clients, workspace] = await Promise.all([getClients(), getWorkspace(params.workspaceId)]);

  const data = configurePipes(workspace.pipe_configs);

  if (!data || !('nodes' in data) || !('edges' in data)) {
    throw new Error('Invalid data format');
  }

  return {
    clients: clients as unknown as Client[],
    data: { nodes: data.nodes as DataNode[], edges: data.edges },
    workspace,
  };
}
