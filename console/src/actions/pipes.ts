import axios from 'axios';
import { getId } from '../utils';
import { PIPE_URL, edgePresets, nodePresets, headers } from '../utils/constants';
import { Node, Position } from 'reactflow';
import { NodeData } from '../types';

async function getPipes() {
  try {
    const response = await axios.get(PIPE_URL, { headers });
    const pipes = response.data.configs;
    if (!response || response.data === undefined || pipes.length === 0) {
      return { nodes: [], edges: [] };
    }
    return configurePipes(pipes);
  } catch (error) {
    console.error(error);
  }
}

type pipeData = {
  id: number;
  workspace_id: number;
  pipe: any[];
};

type createPipeParams = {
  workspaceId: string;
  id: number;
  sourceNodeData: any;
  targetNodeData: any;
};

const createPipe = async ({
  workspaceId,
  id,
  sourceNodeData,
  targetNodeData,
}: createPipeParams) => {
  let method = 'put';
  if (id === 0) {
    method = 'post';
  }

  if (sourceNodeData.type === 'mycelial_server') sourceNodeData.name = 'mycelial_server_source';
  if (targetNodeData.type === 'mycelial_server')
    targetNodeData.name = 'mycelial_server_destination';

  const payload = {
    configs: [
      {
        workspace_id: parseInt(workspaceId),
        id,
        pipe: [sourceNodeData, targetNodeData],
      },
    ],
  };

  try {
    const response = await axios({
      method,
      url: PIPE_URL,
      data: payload,
      headers,
    });
    if (method === 'post') {
      return response.data[0];
    }
    return response.status;
  } catch (error) {
    return error;
  }
};

const deletePipe = async (id: string) => {
  try {
    const response = await axios({
      method: 'delete',
      url: `${PIPE_URL}/${id}`,
      headers,
    });
    return response;
  } catch (error) {
    return error;
  }
};

const configurePipes = (pipes: pipeData[]) => {
  let initialNodes: { [id: string]: Partial<Node> } = {};
  let initialEdges = [];

  let nodemap: { [key: string]: string } = {};

  for (const pipeConfig of pipes) {
    let pipeId = pipeConfig.id;
    let edge = { ...edgePresets };
    edge.data = { id: pipeId };
    edge.id = getId();

    for (const [index, nodeData] of pipeConfig.pipe.entries()) {
      let node: Partial<Node> & {
        key: string;
        data: NodeData;
        sourcePosition?: Position | undefined;
      } = { ...nodePresets };
      const { name, ...data } = nodeData;
      const key = JSON.stringify(data);

      if (nodemap.hasOwnProperty(key)) {
        node.id = nodemap[key];
      } else {
        const id = getId();
        node.id = id;
        nodemap[key] = id;
        initialNodes[id] = {};
      }

      node.data = nodeData;
      node.key = node.id;

      if (index === 0) {
        node.data.source = true;
        node.sourcePosition = Position.Right;
        edge.source = node.id;
      }

      if (index === 1) {
        node.data.destination = true;
        edge.target = node.id;
        node.targetPosition = Position.Left;
      }

      initialNodes[node.id] = node;
    }
    initialEdges.push(edge);
  }

  const data = {
    nodes: Object.values(initialNodes),
    edges: initialEdges,
  };

  return data;
};

export { getPipes, configurePipes, createPipe, deletePipe };
