import { Node, Edge } from 'reactflow';
import DataNode from './components/DataNode';

export type Client = {
  id: string;
  displayName: string;
  name?: string;
  sections: NodeData[];
};
export enum DrawerType {
  Clients = 'clients',
  Edit = 'edit',
}
export enum FlowType {
  Source = 'source',
  Destination = 'destination',
}

type EdgeData = { id: number };
export type DataEdge = Edge<EdgeData>;

export type NodeData =
  | SqlitePhysicalReplicationNodeData
  | SqliteConnectorNodeData
  | PostgresConnectorNodeData
  | MyceliteDestinationNodeData
  | MyceliteSourceNodeData
  | SnowflakeNodeData
  | HelloWorldNodeData
  | ExcelConnectorNodeData
  | KafkaNodeData;

export type BaseNodeData = {
  id?: string;
  display_name: string;
  type: string;
  clientId?: string;
  clientName?: string;
  name: string;
  source?: boolean;
  destination?: boolean;
};

type SqlitePhysicalReplicationNodeData = BaseNodeData & {
  journal_path: string;
  strict: boolean;
  tables: string;
};

type SqliteConnectorNodeData = BaseNodeData & {
  path: string;
  origin: string;
  query: string;
};

type PostgresConnectorNodeData = BaseNodeData & {
  url: string;
};

type MyceliteDestinationNodeData = BaseNodeData & {
  journal_path: string;
  database_path: string;
};

type MyceliteSourceNodeData = BaseNodeData & {
  journal_path: string;
};

type SnowflakeNodeData = BaseNodeData & {
  username: string;
  password: string;
  role: string;
  account_identifier: string;
  warehouse: string;
  database: string;
};

type HelloWorldNodeData = BaseNodeData & {
  message: string;
  interval_milis: number;
};

type ExcelConnectorNodeData = BaseNodeData & {
  tables: string;
  strict: boolean;
  sheets: string;
};

type KafkaNodeData = BaseNodeData & {
  brokers: string;
};

export type DataNode = Node<NodeData>;

export type WorkspaceData = {
  clients: Client[];
  data: { nodes: DataNode[]; edges: Edge[] };
  workspace: Workspace;
};

export type Workspace = {
  id: string;
  created_at: string;
  pipe_configs: [];
  name: string;
};

export type WorkspacesData = Workspace[];

export interface MycelialServerSection {
  client: string;
  destination: boolean;
  display_name: string;
  endpoint: string;
  id: string;
  name: string;
  source: boolean;
  token: string;
  topic: string;
  type: string;
}

export interface SqliteConnector {
  type: string;
  display_name: string;
  path: string;
  truncate: bool;
}

export interface SqlitePhysicalReplication {
  type: string;
  display_name: string;
  journal_path: string;
}

export type ClientResponse = {
  clients: ClientResponseData[];
};

export type ClientResponseData = {
  id: string;
  display_name: string;
  sources?: NodeData[];
  destinations?: NodeData[];
};
