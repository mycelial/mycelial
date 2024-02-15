import { MarkerType } from 'reactflow';
import { getId } from '.';
import DataEdge from '../components/DataEdge';
import DataNode from '../components/DataNode';

export const API_URL = '/api';
export const PIPE_URL = `${API_URL}/pipe`;
export const CLIENT_URL = `${API_URL}/clients`;
export const DAEMON_TOKEN_URL = `${API_URL}/daemon_token`;
export const TOKEN_URL = `${API_URL}/token`;
export const WORKSPACE_URL = `${API_URL}/workspaces`;

export const nodeTypes = {
  dataNode: DataNode,
};

export const edgeTypes = {
  dataEdge: DataEdge,
};

export enum NodeType {
  source = 'source',
  target = 'target',
}

export const headers = {
  'Content-Type': 'application/json',
  Authorization: 'Basic dG9rZW46',
};

export const edgePresets = {
  animated: true,
  id: getId(),
  data: { id: 0 },
  style: {
    stroke: '#188e10',
    strokeWidth: 1,
  },
  markerEnd: {
    type: MarkerType.ArrowClosed,
    width: 12,
    height: 12,
    color: '#3a554c',
  },
  target: '',
  type: 'dataEdge',
  source: '',
};

export const nodePresets = {
  type: 'dataNode',
  id: getId(),
  data: {},
  position: { x: 0, y: 0 },
  key: '',
  targetPosition: '',
  sourcePosition: '',
};

export const fieldNames: { [key: string]: string } = {
  interval_milis: 'number',
  strict: 'boolean',
  truncate: 'boolean',
  message: 'text',
  url: 'text',
  topic: 'text',
  brokers: 'text',
  path: 'text',
  token: 'text',
  endpoint: 'text',
  sheets: 'text',
  journal_path: 'text',
  role: 'text',
  warehouse: 'text',
  schema: 'text',
  database: 'text',
  tables: 'text',
  password: 'text',
  username: 'text',
  account_identifier: 'text',
  query: 'text',
  poll_interval: 'number',
  delay: 'number',
  text: 'text',
  column: 'text',
};

export const mycelialServer = {
  id: '123mycelial',
  displayName: 'Mycelial',
  sections: [
    {
      id: 0,
      display_name: 'Mycelial Server',
      clientName: 'Mycelial',
      clientId: '123mycelial',
      source: true,
      destination: true,
      type: 'mycelial_server',
      name: 'mycelial_server',
      token: 'token',
      topic: 'topic',
      endpoint: 'http://localhost:7777/ingestion',
    },
  ],
};
