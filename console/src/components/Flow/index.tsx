/** @jsxImportSource @emotion/react */
import React, { useRef, useCallback, useEffect, useState } from 'react';
import ReactFlow, {
  ReactFlowProvider,
  Controls,
  Background,
  BackgroundVariant,
  Edge,
  useReactFlow,
  Panel,
  ControlButton,
} from 'reactflow';
import Button from '@mui/material/Button';
import { useLoaderData } from 'react-router-dom';
import { getId } from '../../utils';
import dagre from 'dagre';
import { Client, DrawerType } from '../../types.ts';
import { createPipe, deletePipe } from '../../actions/pipes';
import { WorkspaceData, DataNode as DataNodeType } from '../../types.ts';
import Box from '@mui/material/Box';
import ClientDrawer from '../ClientDrawer';
import WorkspaceAppBar from '../WorkspaceAppBar.tsx';
import EditDrawer from '../EditDrawer.tsx';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import 'reactflow/dist/style.css';
import useFlowStore, { selector } from '../../stores/flowStore.tsx';
import RefreshIcon from '@mui/icons-material/Refresh';
import { nodeTypes, edgeTypes } from '../../utils/constants.ts';

const styles = {
  rfWrapper: {
    height: 'calc(100% - 66px)',
    width: '100%',
    display: 'flex',
    flexGrow: 1,
  },
  reactFlow: {
    background: '#f6f6f6',
    width: '100%',
    height: 300,
  },
};

const nodeWidth = 200;
const nodeHeight = 100;

const Flow: React.FC = () => {
  const rf = useReactFlow();
  const {
    nodes,
    edges,
    edgesToBeDeleted,
    setEdgesToBeDeleted,
    setEdges,
    setActiveNode,
    setEditDrawerOpen,
    editDrawerOpen,
    setNodes,
    onConnect,
    onDragOver,
    onNodesChange,
    handleDrawerClose,
    handleDrawerOpen,
    onEdgesChange,
    onNodeClick,
    clientDrawerOpen,
    updateEdgeAnimation,
    setShowActiveNode,
  } = useFlowStore(selector);
  const { clients, data, workspace } = useLoaderData() as WorkspaceData;
  const [published, setPublished] = useState(false);
  const rfWrapper = useRef<HTMLDivElement>(null);

  const getLayoutedElements = (nodes: DataNodeType[], edges: Edge[]) => {
    const g = new dagre.graphlib.Graph();
    const marginForClientDrawer = Math.floor(document.documentElement.clientWidth * 0.22);

    g.setGraph({
      rankdir: 'LR',
      marginx: 40 + marginForClientDrawer,
      marginy: 40,
      nodesep: 40,
      ranksep: 100,
    });
    g.setDefaultEdgeLabel(() => ({}));

    if (!nodes.length || !edges.length) {
      return { nodes, edges };
    }

    nodes.forEach((node) => {
      g.setNode(node.id, { width: nodeWidth, height: nodeHeight });
    });

    edges.forEach((edge) => {
      g.setEdge(edge.source, edge.target);
    });

    dagre.layout(g);

    nodes.forEach((node) => {
      const nodeWithPosition = g.node(node.id);

      // We are shifting the dagre node position (anchor=center center) to the top left
      // so it matches the React Flow node anchor point (top left).
      node.position = {
        x: nodeWithPosition.x - nodeWithPosition.width / 2,
        y: nodeWithPosition.y - nodeWithPosition.height / 2,
      };
      return node;
    });

    return { nodes, edges };
  };

  const onSave = useCallback(async () => {
    for (const deleted of edgesToBeDeleted) {
      try {
        await deletePipe(deleted);
      } catch (error) {
        console.error('Error:', error);
      }
    }
    setEdgesToBeDeleted([]);

    const currentEdges = [...edges];

    for (const edge of currentEdges) {
      const pipeId = edge.data?.id || 0;
      const sourceNode = nodes.filter((node) => node.id === edge.source)[0];
      const targetNode = nodes.filter((node) => node.id === edge.target)[0];
      if (sourceNode === undefined || targetNode === undefined) return;
      const response = await createPipe({
        workspaceId: workspace.id,
        id: pipeId,
        sourceNodeData: sourceNode.data,
        targetNodeData: targetNode.data,
      });

      if (response === 200) {
        setPublished(true);
        setTimeout(() => setPublished(false), 2000);
        continue;
      }

      if (response.id) {
        updateEdgeAnimation(edge.id, response.id);
        setPublished(true);
        setTimeout(() => setPublished(false), 2000);
      }
    }

    setEdges(currentEdges);
  }, [edgesToBeDeleted, edges, nodes]);

  const onDrop = useCallback(
    (event: React.DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      const display_name = event.dataTransfer.getData('application/reactflow');
      const clientId = event.dataTransfer.getData('text');

      if (rfWrapper === null || rf === null) return;
      if (rfWrapper.current === null) return;
      if (typeof display_name === 'undefined' || !display_name) return;
      if (typeof clientId === 'undefined' || !clientId) return;

      const flowBounds = rfWrapper.current.getBoundingClientRect();
      const position = rf.project({
        x: event.clientX - flowBounds.left,
        y: event.clientY - flowBounds.top,
      });
      const nodeClient = clients.filter((client: Client) => client.id === clientId)[0];
      const origin = nodeClient.sections.filter((node) => node.display_name === display_name)[0];

      const newNode = {
        id: getId(),
        type: 'dataNode',
        position,
        data: {
          clientId: nodeClient.id,
          client: nodeClient.id,
          clientName: nodeClient.displayName,
          ...origin,
        },
      };

      const updated = nodes.concat([newNode]);
      setNodes(updated);
      setActiveNode(newNode);
      setEditDrawerOpen(true);
      setShowActiveNode(true);
    },
    [nodes, edges]
  );

  useEffect(() => {
    setShowActiveNode(false);

    const { nodes: initialNodes, edges: initialEdges } = getLayoutedElements(
      data?.nodes,
      data?.edges
    );
    setNodes([...initialNodes]);
    setEdges([...initialEdges]);
  }, []);

  const onRefresh = useCallback(() => {
    const { nodes: layoutedNodes, edges: layoutedEdges } = getLayoutedElements(nodes, edges);
    setNodes([...layoutedNodes]);
    setEdges([...layoutedEdges]);
  }, [nodes, edges]);

  return (
    <>
      <WorkspaceAppBar
        onPublish={onSave}
        published={published}
        name={workspace && workspace.name ? workspace.name : ''}
      />
      <Box sx={styles.rfWrapper} data-testid="flow" ref={rfWrapper}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          deleteKeyCode={[]}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeClick={onNodeClick}
          onConnect={onConnect}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          onDrop={onDrop}
          onDragOver={onDragOver}
          nodeOrigin={[0, 0]}
          style={styles.reactFlow}
        >
          <Background variant={BackgroundVariant.Lines} gap={10} />
          <ClientDrawer
            onClose={() => handleDrawerClose(DrawerType.Clients)}
            open={clientDrawerOpen}
            clients={clients}
          />
          {!clientDrawerOpen && (
            <Panel position="top-left">
              <Button
                size="small"
                variant="text"
                onClick={() => handleDrawerOpen(DrawerType.Clients)}
                endIcon={<ChevronRightIcon />}
              >
                Clients
              </Button>
            </Panel>
          )}
          <EditDrawer onClose={() => handleDrawerClose(DrawerType.Edit)} open={editDrawerOpen} />
          {!editDrawerOpen && (
            <Panel position="top-right">
              <Button
                variant="text"
                size="small"
                onClick={() => handleDrawerOpen(DrawerType.Edit)}
                startIcon={<ChevronLeftIcon />}
              >
                Edit
              </Button>
            </Panel>
          )}
          <Panel position="bottom-left" style={{ marginBottom: '10px', left: '23%' }}>
            <Controls style={{ display: 'flex' }}>
              <ControlButton onClick={onRefresh}>
                <RefreshIcon />
              </ControlButton>
            </Controls>
          </Panel>
          <svg style={{ height: '25px', width: '25px' }} id="defs">
            <defs>
              <linearGradient id="gradient">
                <stop offset="20%" stopColor="#a5d6a7" />
                <stop offset="95%" stopColor="#3a554c" />
              </linearGradient>
            </defs>
          </svg>
        </ReactFlow>
      </Box>
    </>
  );
};

const FlowWithProvider = () => {
  return (
    <ReactFlowProvider>
      <Flow />
    </ReactFlowProvider>
  );
};

export { Flow };
export default FlowWithProvider;