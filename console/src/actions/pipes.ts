import axios from "axios";
import { getId } from "../utils";
import {
  PIPE_URL,
  edgePresets,
  nodePresets,
  headers,
} from "../utils/constants";
import { Node, Position } from "reactflow";
import { NodeData } from "../types";
import { useAuth0 } from "@auth0/auth0-react";

async function getPipes() {
  let h = {
    ...headers,
    "x-auth0-token": "",
  }
  if (import.meta.env.VITE_USE_AUTH0 === "true") {
    const { getIdTokenClaims } = useAuth0();
    const token = await getIdTokenClaims();

    h = {
      ...headers,
      "x-auth0-token": token?.__raw || "",
    }
  }

  try {
    const response = await axios.get(PIPE_URL, { headers: h });
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

const createPipe = async ({ workspace_id, id, pipe }: pipeData, token: string) => {
  let method = "put";
  if (id === 0 || id === undefined) {
    method = "post";
  }
  if (pipe[0].type === "mycelial_server") {
    pipe[0].name = "mycelial_server_source";
  }
  if (pipe[pipe.length - 1].type === "mycelial_server") {
    pipe[pipe.length - 1].name = "mycelial_server_destination";
  }

  const payload = {
    configs: [
      {
        workspace_id,
        id,
        pipe,
      },
    ],
  };
  try {
    const response = await axios({
      method,
      url: PIPE_URL,
      data: payload,
      headers: {
        'x-auth0-token': token,
        ...headers
      },
    });
    if (method === "post") {
      return response.data[0];
    }
    return response.status;
  } catch (error) {
    return error;
  }
};

const deletePipe = async (id: string, token: string) => {
  try {
    const response = await axios({
      method: "delete",
      url: `${PIPE_URL}/${id}`,
      headers: {'x-auth0-token': token, ...headers},
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
    let source = null;
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
      const { id, ...rest } = data;
      let storedID = id;

      if (nodeData.type === "mycelial_server_source" || nodeData.type === "mycelial_server_destination") {
        nodeData.type = "mycelial_server";
      }

      const key = JSON.stringify(rest);

      // only combine "mycelial_server" nodes -- as a kind of visual syntactic sugar.
      if (nodemap.hasOwnProperty(key) && nodeData.type === "mycelial_server") {
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
        node.data.id = pipeId;
      }
      if (node.data.source === true) {
        node.sourcePosition = Position.Right;
        edge.source = node.id;
      }
      if (node.data.destination === true) {
        node.targetPosition = Position.Left;
        edge.target = node.id;
      }

      initialNodes[node.id] = node;

      if (source !== null) {
        const e = { ...edge };
        e.id = getId("edge");
        e.source = source.id || "0";
        e.target = node.id;
        initialEdges.push(e);
      }
      source = node;
    }
  }

  const data = {
    nodes: Object.values(initialNodes),
    edges: initialEdges,
  };

  return data;
};

export { getPipes, configurePipes, createPipe, deletePipe };
