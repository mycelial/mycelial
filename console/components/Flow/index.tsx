"use client";

import { useCallback, useContext, useState, useRef, useEffect } from "react";

import dagre from "dagre";

import Image from "next/image";
import ReactFlow, {
  MarkerType,
  Node,
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  Edge,
  ConnectionLineType,
  MiniMap,
  Controls,
  Background,
  ReactFlowProvider,
  ReactFlowInstance,
  EdgeChange,
  EdgeRemoveChange,
  Panel,
} from "reactflow";
import CustomNode from "./CustomNode";

import { IconDatabase } from "@tabler/icons-react";

import styles from "./Flow.module.css";
// import DatabaseNode from './DatabaseNode';

import {
  SqliteConnectorSourceNode,
  SqliteConnectorDestinationNode,
  SqlitePhysicalReplicationSourceNode,
  SqlitePhysicalReplicationDestinationNode,
  MycelialServerNode,
  KafkaDestination,
  KafkaSourceNode,
  SnowflakeSourceNode,
  SnowflakeDestinationNode,
  PostgresSourceNode,
  HelloWorldSourceNode,
  HelloWorldDestinationNode,
  BacalhauNode,
  BacalhauSourceNode,
  BacalhauDestinationNode,
} from "@/components/nodes";

import { Grid, Group } from "@/components/layout";

import { Button, Box } from "@/components/core";

import { createStyles, Navbar, Text, rem } from "@/components/core";

import ClientProvider, { ClientContext } from "../context/clientContext";
import {
  ClientContextType,
  IClient,
  IDestination,
  ISource,
} from "../@types/client";

const useStyles = createStyles((theme) => ({
  navbar: {
    paddingTop: 0,
  },

  section: {
    marginLeft: `calc(${theme.spacing.md} * -1)`,
    marginRight: `calc(${theme.spacing.md} * -1)`,
    marginBottom: theme.spacing.md,

    "&:not(:last-of-type)": {
      borderBottom: `${rem(1)} solid ${theme.colorScheme === "dark"
        ? theme.colors.dark[4]
        : theme.colors.gray[3]
        }`,
    },
  },

  mainLinks: {
    paddingLeft: `calc(${theme.spacing.md} - ${theme.spacing.xs})`,
    paddingRight: `calc(${theme.spacing.md} - ${theme.spacing.xs})`,
    paddingBottom: theme.spacing.md,
  },

  mainLink: {
    display: "flex",
    alignItems: "center",
    width: "100%",
    fontSize: theme.fontSizes.xs,
    padding: `${rem(8)} ${theme.spacing.xs}`,
    borderRadius: theme.radius.sm,
    fontWeight: 500,
    color:
      theme.colorScheme === "dark"
        ? theme.colors.dark[0]
        : theme.colors.gray[7],

    "&:hover": {
      backgroundColor:
        theme.colorScheme === "dark"
          ? theme.colors.dark[6]
          : theme.colors.gray[0],
      color: theme.colorScheme === "dark" ? theme.white : theme.black,
    },
  },

  mainLinkInner: {
    display: "flex",
    alignItems: "center",
    flex: 1,
  },

  mainLinkIcon: {
    marginRight: theme.spacing.sm,
    color:
      theme.colorScheme === "dark"
        ? theme.colors.dark[2]
        : theme.colors.gray[6],
  },

  mainLinkBadge: {
    padding: 0,
    width: rem(20),
    height: rem(20),
    pointerEvents: "none",
  },

  collections: {
    paddingLeft: `calc(${theme.spacing.md} - ${rem(6)})`,
    paddingRight: `calc(${theme.spacing.md} - ${rem(6)})`,
    paddingBottom: theme.spacing.md,
  },

  collectionsHeader: {
    paddingLeft: `calc(${theme.spacing.md} + ${rem(2)})`,
    paddingRight: theme.spacing.md,
    marginBottom: rem(5),
  },

  collectionLink: {
    display: "block",
    padding: `${rem(8)} ${theme.spacing.xs}`,
    textDecoration: "none",
    borderRadius: theme.radius.sm,
    fontSize: theme.fontSizes.xs,
    color:
      theme.colorScheme === "dark"
        ? theme.colors.dark[0]
        : theme.colors.gray[7],
    lineHeight: 1,
    fontWeight: 500,
    cursor: "grab",

    "&:hover": {
      backgroundColor:
        theme.colorScheme === "dark"
          ? theme.colors.dark[6]
          : theme.colors.gray[0],
      color: theme.colorScheme === "dark" ? theme.white : theme.black,
    },
  },
}));

const initialNodes: Node[] = [];

const initialEdges: Edge[] = [];

const nodeTypes = {
  custom: CustomNode,
  sqlite_connector_source: SqliteConnectorSourceNode,
  sqlite_connector_destination: SqliteConnectorDestinationNode,
  sqlite_physical_replication_source: SqlitePhysicalReplicationSourceNode,
  sqlite_physical_replication_destination: SqlitePhysicalReplicationDestinationNode,
  mycelial_server: MycelialServerNode,
  kafka_source: KafkaSourceNode,
  kafka_destination: KafkaDestination,
  snowflake_source: SnowflakeSourceNode,
  snowflake_destination: SnowflakeDestinationNode,
  postgres_source: PostgresSourceNode,
  hello_world_source: HelloWorldSourceNode,
  hello_world_destination: HelloWorldDestinationNode,
  bacalhau: BacalhauNode,
  bacalhau_source: BacalhauSourceNode,
  bacalhau_destination: BacalhauDestinationNode,
};

const defaultEdgeOptions = {
  animated: false,
  type: "smoothstep",
  markerEnd: {
    type: MarkerType.ArrowClosed,
  },
};

const getRandomString = () => {
  return String(Date.now().toString(32) + Math.random().toString(16)).replace(
    /\./g,
    "",
  );
};
const getId = () => `dndnode_${getRandomString()}`;

type NavbarSearchProps = {
  onSave: () => void;
};

function NavbarSearch(props: NavbarSearchProps) {
  const { classes } = useStyles();
  const { clients } = (useContext(ClientContext) as ClientContextType) || {};

  const onDragStart = (
    event: any,
    client: IClient | null,
    source: ISource | null,
    destination: IDestination | null,
  ) => {
    // fixme: better way to pass state through than the stringified json?
    event.dataTransfer.setData(
      "application/json",
      JSON.stringify({
        client: client,
        source: source,
        destination: destination,
      }),
    );
    event.dataTransfer.effectAllowed = "move";
  };

  const sourcesLinks = clients.flatMap((client) =>
    client.sources.map((source, idx) => {
      const label = `Source: ${client.display_name} - ${source.display_name}`;

      return (
        <div
          className={classes.collectionLink}
          key={label}
          onDragStart={(event) => onDragStart(event, client, source, null)}
          draggable
        >
          {label}
        </div>
      );
    }),
  );

  const destinationsLinks = clients.flatMap((client) =>
    client.destinations.map((dest, idx) => {
      const label = `Destination: ${client.display_name} - ${dest.display_name}`;

      return (
        <div
          className={classes.collectionLink}
          key={label}
          onDragStart={(event) => onDragStart(event, client, null, dest)}
          draggable
        >
          {label}
        </div>
      );
    }),
  );

  // todo: is there a data-driven way to do this?
  const transportLinks = () => {
    let source = {
      type: "mycelial_server",
      display_name: "Mycelial Server",
      endpoint: "http://localhost:8080/ingestion",
      token: "token",
      topic: getRandomString(),
    };

    let bac_source = {
      type: "bacalhau",
      display_name: "Bacalhau",
      endpoint: "http://localhost:2112/accept",
      job: "Sample",
    };

    return [
      <div
        key="mycelial_server"
        className={classes.collectionLink}
        onDragStart={(event) => onDragStart(event, null, source, source)}
        draggable
      >
        Mycelial Server
      </div>,
      <div
        key="bacalhau"
        className={classes.collectionLink}
        onDragStart={(event) => onDragStart(event, null, bac_source, bac_source)}
        draggable
      >
        Bacalhau Node
      </div>,
    ];
  };

  return (
    <Navbar
      height="100vh"
      width={{ sm: 200 }}
      p="md"
      className={classes.navbar}
    >
      <Image
        className="p-1 mb-2"
        src="/mycelial.svg"
        alt="Mycelial Logo"
        width={217}
        height={47}
        priority
      />
      <Navbar.Section className={classes.section}>
        <Group className={classes.collectionsHeader} position="apart">
          <Text size="xs" weight={500} color="dimmed">
            Available Nodes
          </Text>
        </Group>
        <div className={classes.collections}>{sourcesLinks}</div>
        <div className={classes.collections}>{transportLinks()}</div>
        <div className={classes.collections}>{destinationsLinks}</div>
        <div>
          <Box style={{ padding: "1rem" }}>
            <Button
              variant="light"
              color="aqua"
              className="drop-shadow-md bg-blue-50"
              fullWidth
              leftIcon={<IconDatabase size="1rem" />}
              onClick={props.onSave}
            >
              Publish
            </Button>
          </Box>
        </div>
      </Navbar.Section>
    </Navbar>
  );
}

async function getConfigs(token: string) {
  try {
    const response = await fetch("/api/pipe", {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
        "X-Authorization": "Bearer " + btoa(token),
      },
    });
    const result = await response.json();
    return result;
  } catch (error) { }
}

const dagreGraph = new dagre.graphlib.Graph();
dagreGraph.setDefaultEdgeLabel(() => ({}));

const getLayoutedElements = (nodes: any[], edges: any[], direction = "TB") => {
  const isHorizontal = direction === "LR";
  dagreGraph.setGraph({ rankdir: direction });

  nodes.forEach((node) => {
    dagreGraph.setNode(node.id, { width: node.width, height: node.height });
  });

  edges.forEach((edge) => {
    dagreGraph.setEdge(edge.source, edge.target);
  });

  dagre.layout(dagreGraph);

  nodes.forEach((node) => {
    const nodeWithPosition = dagreGraph.node(node.id);
    node.targetPosition = isHorizontal ? "left" : "top";
    node.sourcePosition = isHorizontal ? "right" : "bottom";

    // We are shifting the dagre node position (anchor=center center) to the top left
    // so it matches the React Flow node anchor point (top left).
    node.position = {
      x: nodeWithPosition.x - node.width / 2,
      y: nodeWithPosition.y - node.height / 2,
    };

    return node;
  });

  return { nodes, edges };
};

const { nodes: layoutedNodes, edges: layoutedEdges } = getLayoutedElements(
  initialNodes,
  initialEdges,
);

function Flow() {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);

  const [nodes, setNodes, onNodesChange] = useNodesState(layoutedNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(layoutedEdges);
  const [edgesToBeDeleted, setEdgesToBeDeleted] = useState<number[]>([]);
  const [initialDataLoaded, setInitialDataLoaded] = useState(false);

  const onConnect = useCallback(
    (params: Connection | Edge) => setEdges((eds) => addEdge(params, eds)),
    [setEdges],
  );

  const { token } = (useContext(ClientContext) as ClientContextType) || {};

  const loadConfig = useCallback(async () => {
    let configs = await getConfigs(token);

    let nodeMap = new Map<string, Node>();

    for (const config of configs.configs) {
      let source = null;
      for (const element of config.pipe) {
        let { name, ...data } = element;
        const id = getId();

        let nodeType = element.name;
        if (
          nodeType === "mycelial_server_source" ||
          nodeType === "mycelial_server_destination"
        ) {
          nodeType = "mycelial_server";
        }

        if (
          nodeType === "bacalhau_source" ||
          nodeType === "bacalhaU_destination"
        ) {
          nodeType = "bacalhau";
        }

        let node: Node = {
          id: "temp",
          type: nodeType,
          position: {
            x: 0,
            y: 0,
          },
          data: data,
        };

        let actualNode = {
          ...node,
          id,
        };

        const key = JSON.stringify(node);

        if (nodeMap.has(key)) {
          let existingNode = nodeMap.get(key);
          if (existingNode === undefined) {
            continue;
          }
          actualNode = {
            ...node,
            id: existingNode.id,
          };
        }

        nodeMap.set(key, actualNode);
        setNodes((nds) => nds.concat(actualNode));

        if (source !== null) {
          let edge = {
            id: getId(),
            source: source.id,
            target: actualNode.id,
            animated: false,
            type: "smoothstep",
            markerEnd: {
              type: MarkerType.ArrowClosed,
            },
            data: {
              id: config.id,
            },
          };
          setEdges((eds) => eds.concat(edge));
        }

        source = actualNode;
      }
    }
  }, [setEdges, setNodes, token]);

  const onLayout = useCallback(
    (direction: string | undefined) => {
      const { nodes: layoutedNodes, edges: layoutedEdges } =
        getLayoutedElements(nodes, edges, direction);

      setNodes([...layoutedNodes]);
      setEdges([...layoutedEdges]);
    },
    [nodes, edges],
  );

  const setInitialElements = useCallback(async () => {
    await loadConfig();

    await new Promise((r) => setTimeout(r, 100));
    setInitialDataLoaded(true);
  }, [setEdges, setNodes, token]);

  useEffect(() => {
    setInitialElements();
  }, [setInitialElements]);

  const [reactFlowInstance, setReactFlowInstance] =
    useState<ReactFlowInstance | null>(null);

  useEffect(() => {
    onLayout("LR");
  }, [initialDataLoaded]);

  const onEdgeChange = useCallback(
    (eds: EdgeChange[]) => {
      for (const eid in eds) {
        const change = eds[eid];
        if (change.type === "remove") {
          let edgeChange = change as EdgeRemoveChange;
          let storedEdgeId = reactFlowInstance?.getEdge(edgeChange.id)?.data?.id;
          setEdgesToBeDeleted((eds) => eds.concat([storedEdgeId]));
        }
      }
      onEdgesChange(eds);
    },
    [reactFlowInstance, onEdgesChange, setEdgesToBeDeleted, edgesToBeDeleted],
  );

  const getDetailsForNode = useCallback(
    (node: Node | undefined, kind: String) => {
      if (node?.type === "mycelial_server" && kind === "source") {
        return {
          name: "mycelial_server_source",
          ...node.data,
        };
      } else if (node?.type === "mycelial_server" && kind === "destination") {
        return {
          name: "mycelial_server_destination",
          ...node.data,
        };
      } else if (node?.type === "bacalhau" && kind === "source") {
        return {
          name: "bacalhau_source",
          ...node.data,
        };
      } else if (node?.type === "bacalhau" && kind === "destination") {
        return {
          name: "bacalhau_destination",
          ...node.data,
        }
      } else if (node?.type) {
        return {
          name: node.type,
          ...node.data,
        };
      }
    },
    [],
  );

  const handleSave = useCallback(async () => {
    let new_configs = [];
    let configs = [];

    if (reactFlowInstance === null) {
      return;
    }

    let toDelete = edgesToBeDeleted;

    const rf = reactFlowInstance;

    for (const edge of edges) {
      let pipe = [];

      const sourceNode = rf.getNode(edge.source);
      const targetNode = rf.getNode(edge.target);

      const sourceSection = getDetailsForNode(sourceNode, "source");
      const targetSection = getDetailsForNode(targetNode, "destination");

      if (sourceSection === undefined || targetSection === undefined) {
        continue;
      }

      pipe.push(sourceSection);
      pipe.push(targetSection);

      if (edge.data?.id) {
        let payload = {
          configs: [{ id: edge.data.id, pipe: pipe }]
        }
        try {
          const response = await fetch("/api/pipe", {
            method: "PUT",
            headers: {
              "Content-Type": "application/json",
              "X-Authorization": "Bearer " + btoa(token),
            },
            body: JSON.stringify(payload),
          });

          await response.json();
        } catch (error) {
          console.error("Error:", error);
        }
      } else {
        let id = 0;
        let payload = {
          configs: [{ id: 0, pipe: pipe }]
        }
        const response = await fetch("/api/pipe", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Authorization": "Bearer " + btoa(token),
          },
          body: JSON.stringify(payload),
        });

        const result = await response.json();
        id = result[0].id;

        rf.setEdges((eds) => {
          return eds.map((ed) => {
            if (ed.id === edge.id) {
              return {
                ...ed,
                data: {
                  id: id,
                },
              }
            }
            return ed;
          });
        })
      }
    }

    for (const key in toDelete) {
      try {
        const response = await fetch(
          `/api/pipe/${edgesToBeDeleted[key]}`,
          {
            method: "DELETE",
            headers: {
              "Content-Type": "application/json",
              "X-Authorization": "Bearer " + btoa(token),
            },
          },
        );
      } catch (error) {
        console.error("Error:", error);
      }
    }
    setEdgesToBeDeleted([]);
  }, [
    edges,
    reactFlowInstance,
    token,
    getDetailsForNode,
    edgesToBeDeleted,
    setEdgesToBeDeleted,
  ]);

  const onDragOver = useCallback((event: any) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
  }, []);

  const onDrop = useCallback(
    (event: any) => {
      if (reactFlowWrapper === null || reactFlowInstance === null) {
        return;
      }

      if (reactFlowWrapper.current === null) {
        return;
      }

      event.preventDefault();

      const reactFlowBounds = reactFlowWrapper.current.getBoundingClientRect();
      const {
        client: client,
        source: source,
        destination: destination,
      } = JSON.parse(event.dataTransfer.getData("application/json"));

      // check if the dropped element is valid
      if (
        (typeof client === "undefined" || !client) &&
        (!source || !destination)
      ) {
        return;
      }

      const position = reactFlowInstance.project({
        x: event.clientX - reactFlowBounds.left,
        y: event.clientY - reactFlowBounds.top,
      });

      let newNode: Node;
      if (source && destination) {
        const type = `${source.type}`;
        newNode = {
          id: getId(),
          type,
          position,
          data: { label: `${type} node`, ...source },
        };
      } else if (source) {
        const type = `${source.type}_source`;
        newNode = {
          id: getId(),
          type,
          position,
          data: { label: `${type} node`, client: client.id, ...source },
        };
      } else if (destination) {
        const type = `${destination.type}_destination`;
        newNode = {
          id: getId(),
          type,
          position,
          data: { label: `${type} node`, client: client.id, ...destination },
        };
      } else {
        console.error(
          "either source or destination should be set on the drag element",
        );
        return;
      }

      setNodes((nds) => nds.concat(newNode));
    },
    [reactFlowInstance, setNodes],
  );

  const minimapStyle = {
    height: 120,
  };

  return (
    <ReactFlowProvider>
      <ClientProvider>
        <Grid gutter={0}>
          <Grid.Col span="content">
            <NavbarSearch onSave={handleSave} />
          </Grid.Col>
          <Grid.Col span="auto">
            <div className={styles.flow} ref={reactFlowWrapper}>
              <ReactFlow
                nodes={nodes}
                onNodesChange={onNodesChange}
                edges={edges}
                onEdgesChange={onEdgeChange}
                onConnect={onConnect}
                nodeTypes={nodeTypes}
                defaultEdgeOptions={defaultEdgeOptions}
                connectionLineType={ConnectionLineType.SmoothStep}
                onInit={setReactFlowInstance}
                onDrop={onDrop}
                onDragOver={onDragOver}
                snapToGrid={true}
              >
                <Controls />
                <Background color="#c8cbcc" gap={16} />

                <Panel position="top-right">
                  {/* <button className="text-blue-300 bg-blue-50 rounded p-2 drop-shadow-md hover:bg-blue-100" onClick={() => onLayout('LR')}>auto-layout</button> */}
                  {/* <button className="text-blue-300 bg-blue-50 rounded p-2 drop-shadow-md hover:bg-blue-100" onClick={() => removeMycelialServerNodes()}>remove myc net nodes</button> */}
                </Panel>
              </ReactFlow>
            </div>
          </Grid.Col>
        </Grid>
      </ClientProvider>
    </ReactFlowProvider>
  );
}

export default Flow;
