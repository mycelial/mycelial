import { MouseEvent, DragEvent } from 'react';
import { DataNode } from '../types';
import { getId } from '../utils';
import { edgePresets } from '../utils/constants';
import { create } from 'zustand';
import {
  Connection,
  Edge,
  EdgeChange,
  Node,
  NodeChange,
  addEdge,
  OnNodesChange,
  OnEdgesChange,
  OnConnect,
  applyNodeChanges,
  applyEdgeChanges,
} from 'reactflow';

type RFState = {
  nodes: Node[];
  edges: Edge[];
  edgesToBeDeleted: string[];
  addEdgeToBeDeleted: (pipeId: string) => void;
  setEdgesToBeDeleted: (edges: string[]) => void;
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  onConnect: OnConnect;
  setNodes: (nodes: Node[]) => void;
  setEdges: (edges: Edge[]) => void;
  onDragOver: (event: React.DragEvent<HTMLDivElement>) => void;
  activeNode: null | DataNode;
  setActiveNode: (node: DataNode | null | undefined) => void;
  onNodeClick: (
    event: MouseEvent<HTMLElement, MouseEvent<Element, MouseEvent>>,
    node: DataNode
  ) => void;
  setEditDrawerOpen: (open: boolean) => void;
  editDrawerOpen: boolean;
  setClientDrawerOpen: (open: boolean) => void;
  clientDrawerOpen: boolean;
  showActiveNode: boolean;
  setShowActiveNode: (show: boolean) => void;
  unconnectedNodes: DataNode[];
  addUnconnectedNode: (id: string) => void;
  setUnconnectedNodes: (ids: DataNode[]) => void;
  noEdits: boolean;
  setNoEdits: (noEdits: boolean) => void;
  deletedNodes: Node[];
  setDeletedNodes: (nodes: Node[]) => void;
  addNodeToBeDeleted: (deleted: Node | undefined) => void;
  getNode: (id: string) => Node | undefined;
  handleDrawerOpen: (identifier: string) => void;
  handleDrawerClose: (identifier: string) => void;
  updateEdgeAnimation: (edgeId: string, dataId: string) => void;
  getEdge: (id: string) => Edge | undefined;
};

const useFlowStore = create<RFState>((set, get) => ({
  activeNode: null,
  clientDrawerOpen: true,
  editDrawerOpen: true,
  showActiveNode: false,
  noEdits: true,
  deletedNodes: [],
  nodes: [],
  edges: [],
  edgesToBeDeleted: [],
  unconnectedNodes: [],
  setDeletedNodes: (nodes: Node[]) => set({ deletedNodes: nodes }),
  getNode: (id: string) => get().nodes.filter((node) => node.id === id)[0],
  getEdge: (id: string) => get().edges.filter((edge) => edge.id === id)[0],
  addNodeToBeDeleted: (deleted: Node | undefined) => {
    if (!deleted) return;
    return set({ deletedNodes: get().deletedNodes.concat([deleted]) });
  },
  setNoEdits: (noEdits: boolean) => set({ noEdits }),
  setClientDrawerOpen: (open: boolean) => {
    set({ clientDrawerOpen: open });
  },
  setEditDrawerOpen: (open: boolean) => {
    set({ editDrawerOpen: open });
  },
  setShowActiveNode: (show: boolean) => {
    set({ showActiveNode: show });
  },
  setActiveNode: (node: DataNode | null | undefined) => {
    set({ activeNode: node });
  },
  setNodes: (nodes: Node[]) => {
    set({ nodes });
  },
  setEdges: (edges: Edge[]) => set({ edges }),
  addNode: (node: Node) => {
    get().nodes.concat(node);
  },
  addEdgeToBeDeleted: (pipeId: string) => {
    const deletedEdges = get().edgesToBeDeleted.concat([pipeId]);
    set({ edgesToBeDeleted: deletedEdges });
  },
  updateEdgeAnimation: (edgeId: string, dataId: string) => {
    const updatedEdges = get().edges.map((edge) => {
      if (edge.id === edgeId) {
        edge.data.id = dataId;
        edge.animated = true;
      }
      return edge;
    });
    return set({ edges: updatedEdges });
  },
  setEdgesToBeDeleted: (edges: string[]) => set({ edgesToBeDeleted: edges }),
  onNodesChange: (changes: NodeChange[]) => {
    set({
      nodes: applyNodeChanges(changes, get().nodes),
    });
  },
  addUnconnectedNode: (id: string) => {
    const unconnected = get().getNode(id);
    if (!unconnected) return;
    set({ unconnectedNodes: get().unconnectedNodes.concat(unconnected) });
  },
  setUnconnectedNodes: (ids: DataNode[]) => set({ unconnectedNodes: ids }),
  onEdgesChange: (edgeChanges: EdgeChange[]) => {
    set({
      edges: applyEdgeChanges(edgeChanges, get().edges),
    });
  },
  onConnect: (connection: Connection) => {
    const newEdge = {
      ...edgePresets,
      id: getId(),
      animated: false,
      ...connection,
      data: { id: 0 },
    };

    set({
      edges: addEdge(newEdge, get().edges),
    });
  },
  onDragOver: (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  },
  onNodeClick: (_: MouseEvent<HTMLElement, MouseEvent<Element, MouseEvent>>, node: DataNode) => {
    // onNodeClick runs after onNodesDelete so return to
    // avoid setting deleted node to activeNode
    get().setActiveNode(node);
    get().setShowActiveNode(true);
    get().setEditDrawerOpen(true);
  },
  handleDrawerOpen: (identifier: string): void =>
    ((
      {
        clients: () => get().setClientDrawerOpen(true),
        edit: () => get().setEditDrawerOpen(true),
      } as { [key: string]: () => void }
    )[identifier]()),
  handleDrawerClose: (identifier: string): void =>
    ((
      {
        clients: () => get().setClientDrawerOpen(false),
        edit: () => get().setEditDrawerOpen(false),
        undefined: () => undefined,
      } as { [key: string]: () => void }
    )[identifier]()),
}));

export const selector = (store: RFState) => ({
  nodes: store.nodes,
  edges: store.edges,
  edgesToBeDeleted: store.edgesToBeDeleted,
  activeNode: store.activeNode,
  clientDrawerOpen: store.clientDrawerOpen,
  editDrawerOpen: store.editDrawerOpen,
  showActiveNode: store.showActiveNode,
  unconnectedNodes: store.unconnectedNodes,
  deletedNodes: store.deletedNodes,
  setEdges: store.setEdges,
  setNodes: store.setNodes,
  onNodesChange: store.onNodesChange,
  onEdgesChange: store.onEdgesChange,
  onDragOver: store.onDragOver,
  onConnect: store.onConnect,
  setEdgesToBeDeleted: store.setEdgesToBeDeleted,
  setActiveNode: store.setActiveNode,
  setClientDrawerOpen: store.setClientDrawerOpen,
  setEditDrawerOpen: store.setEditDrawerOpen,
  setShowActiveNode: store.setShowActiveNode,
  onNodeClick: store.onNodeClick,
  addNodeToBeDeleted: store.addNodeToBeDeleted,
  handleDrawerOpen: store.handleDrawerOpen,
  handleDrawerClose: store.handleDrawerClose,
  updateEdgeAnimation: store.updateEdgeAnimation,
  addEdgeToBeDeleted: store.addEdgeToBeDeleted,
  getNode: store.getNode,
  addUnconnectedNode: store.addUnconnectedNode,
  getEdge: store.getEdge,
});

export default useFlowStore;
