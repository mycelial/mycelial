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
  SqliteSourceNode,
  SqliteDestinationNode,
  MycelialNetworkNode,
  KafkaSourceNode,
  SnowflakeSourceNode,
  SnowflakeDestinationNode,
} from "@/components/nodes";

import { Grid, Group } from "@/components/layout";

import { Button, Box } from "@/components/core";

import { createStyles, Navbar, Text, rem } from "@/components/core";

import ClientProvider, { ClientContext } from "../context/clientContext";
import {ClientContextType, IClient, IDestination, ISource} from "../@types/client";

const useStyles = createStyles((theme) => ({
  navbar: {
    paddingTop: 0,
  },

  section: {
    marginLeft: `calc(${theme.spacing.md} * -1)`,
    marginRight: `calc(${theme.spacing.md} * -1)`,
    marginBottom: theme.spacing.md,

    "&:not(:last-of-type)": {
      borderBottom: `${rem(1)} solid ${
        theme.colorScheme === "dark"
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

const collections = [
  { label: "Sqlite Source", nodeType: "sqliteSource" },
  { label: "Sqlite Destination", nodeType: "sqliteDestination" },
  { label: "Mycelial Network", nodeType: "mycelialNetwork" },
  { label: "Kafka Source", nodeType: "kafkaSource" },
  { label: "Snowflake Source", nodeType: "snowflakeSource" },
  { label: "Snowflake Destination", nodeType: "snowflakeDestination" },
];

const initialNodes: Node[] = [];

const initialEdges: Edge[] = [];

const nodeTypes = {
  custom: CustomNode,
  sqliteSource: SqliteSourceNode,
  sqliteDestination: SqliteDestinationNode,
  mycelialNetwork: MycelialNetworkNode,
  kafkaSource: KafkaSourceNode,
  snowflakeSource: SnowflakeSourceNode,
  snowflakeDestination: SnowflakeDestinationNode,
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
    ""
  );
};
const getId = () => `dndnode_${getRandomString()}`;

type NavbarSearchProps = {
  onSave: () => void;
};

function NavbarSearch(props: NavbarSearchProps) {
  const { classes } = useStyles();
  const ctx = (useContext(ClientContext) as ClientContextType) || {};
  const { clients } = ctx;

  const onDragStart = (event: any, client: IClient, source: ISource | null, destination: IDestination | null) => {
    // fixme: better way to pass state through than the stringified json?
    event.dataTransfer.setData("application/json", JSON.stringify({
      client: client,
      source: source,
      destination: destination,
    }));
    event.dataTransfer.effectAllowed = "move";
  };

  const sourcesLinks = ctx.clients.flatMap((client) => (
      client.sources.map((source, idx) => {
        const label = `Source: ${client.display_name} - ${source.display_name}`;

        return <div
            className={classes.collectionLink}
            key={label}
            onDragStart={(event) => onDragStart(event, client, source, null)}
            draggable
        >
          {label}
        </div>;
      })
  ));

  const destinationsLinks = ctx.clients.flatMap((client) => (
      client.destinations.map((dest, idx) => {
        const label = `Destination: ${client.display_name} - ${dest.display_name}`;

        return <div
            className={classes.collectionLink}
            key={label}
            onDragStart={(event) => onDragStart(event, client, null, dest)}
            draggable
        >
          {label}
        </div>;
      })
  ));

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
    const response = await fetch("/api/pipe/configs", {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
        "X-Authorization": "Bearer " + btoa(token),
      },
    });
    const result = await response.json();
    return result;
  } catch (error) {}
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
  initialEdges
);

function Flow() {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);

  const [nodes, setNodes, onNodesChange] = useNodesState(layoutedNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(layoutedEdges);
  const [edgesToBeDeleted, setEdgesToBeDeleted] = useState<number[]>([]);
  const [initialDataLoaded, setInitialDataLoaded] = useState(false);

  const onConnect = useCallback(
    (params: Connection | Edge) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  const { token } = (useContext(ClientContext) as ClientContextType) || {};

  const loadConfig = useCallback(async () => {
    let configs = await getConfigs(token);

    const nodeTypes = (name: string) => {
      let nt = new Map<string, string>([
        ["sqlite_source", "sqliteSource"],
        ["sqlite_destination", "sqliteDestination"],
        ["mycelial_net_source", "mycelialNetwork"],
        ["mycelial_net_destination", "mycelialNetwork"],
        ["kafka_source", "kafkaSource"],
        ["snowflake_source", "snowflakeSource"],
        ["snowflake_destination", "snowflakeDestination"],
      ]);
      return nt.get(name);
    };

    let nodeMap = new Map<string, Node>();

    for (const config of configs.configs) {
      let source = null;
      for (const element of config.pipe.section) {
        let { name, ...data } = element;
        const id = getId();
        let node: Node = {
          id: "temp",
          type: nodeTypes(element.name),
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
              ids: [config.id],
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
    [nodes, edges]
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

  const removeMycelialNetworkNodes = useCallback(() => {
    // let nodesToDelete: Node[] = [];
    setNodes((nds) => {
      nds.forEach((nd: Node) => {
        if (nd.type === "mycelialNetwork") {
          // nodesToDelete.push(nd);
          reactFlowInstance?.getEdges().forEach((ed) => {
            if (ed.source === nd.id) {
              reactFlowInstance?.getEdges().forEach((ed2) => {
                if (ed2.target === nd.id) {
                  // create a new edge between the source and target connected to the mycelial network
                  // node so that the flow is the same once the mycelial network node is removed
                  setEdges((eds) => {
                    return eds.filter((e) => {
                      return e.id !== ed.id && e.id !== ed2.id;
                    }).concat({
                      id: getId(),
                      source: ed2.source,
                      target: ed.target,
                      animated: false,
                      type: "smoothstep",
                      markerEnd: {
                        type: MarkerType.ArrowClosed,
                      },
                      data: {
                        ids: [ed2.data.ids[0], ed.data.ids[0]],
                        myc_network_info: nd.data,
                      },
                    });
                  });
                }
              });
            }
          });
        }
      });
      return nds.filter((nd) => {
        return nd.type !== "mycelialNetwork";
      });
    });
    // reactFlowInstance?.deleteElements({nodes: nodesToDelete});
  }, [setNodes, reactFlowInstance, setEdges]);

  useEffect(() => {
    onLayout("LR");
    removeMycelialNetworkNodes();
  }, [initialDataLoaded]);


  const onEdgeChange = useCallback(
    (eds: EdgeChange[]) => {
      for (const eid in eds) {
        const change = eds[eid];
        if (change.type === "remove") {
          let edgeChange = change as EdgeRemoveChange;
          let storedEdgeIds = reactFlowInstance?.getEdge(edgeChange.id)?.data?.ids;
          setEdgesToBeDeleted((eds) => eds.concat(storedEdgeIds));
        }
      }
      onEdgesChange(eds);
    },
    [reactFlowInstance, onEdgesChange, setEdgesToBeDeleted, edgesToBeDeleted]
  );

  const getDetailsForNode = useCallback(
    (node: Node | undefined, kind: String) => {
      if (node?.type === "sqliteSource") {
        return {
          name: "sqlite_source",
          client: node.data.client,
          path: node.data.path,
          tables: node.data.tables,
        };
      }

      if (node?.type === "snowflakeSource") {
        return {
          name: "snowflake_source",
          username: node.data.username,
          password: node.data.password,
          role: node.data.role,
          account_identifier: node.data.account_identifier,
          warehouse: node.data.warehouse,
          database: node.data.database,
          schema: node.data.schema,
          query: node.data.query,
          client: node.data.client,
        };
      }

      if (node?.type === "kafkaSource") {
        return {
          name: "kafka_source",
          client: node.data.client,
          brokers: node.data.brokers,
          group_id: node.data.group_id,
          topics: node.data.topics,
        };
      }

      if (node?.type === "sqliteDestination") {
        return {
          name: "sqlite_destination",
          path: node.data.path,
          client: node.data.client,
        };
      }

      if (node?.type === "mycelialNetwork" && kind === "source") {
        return {
          name: "mycelial_net_source",
          endpoint: node.data.endpoint,
          token: node.data.token,
          topic: node.data.topic,
        };
      }

      if (node?.type === "mycelialNetwork" && kind === "destination") {
        return {
          name: "mycelial_net_destination",
          endpoint: node.data.endpoint,
          token: node.data.token,
          topic: node.data.topic,
        };
      }

      if (node?.type === "snowflakeDestination") {
        return {
          name: "snowflake_destination",
          username: node.data.username,
          password: node.data.password,
          role: node.data.role,
          account_identifier: node.data.account_identifier,
          warehouse: node.data.warehouse,
          database: node.data.database,
          schema: node.data.schema,
          table: node.data.table,
        };
      }
    },
    []
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
      let section = [];

      const sourceNode = rf.getNode(edge.source);
      const targetNode = rf.getNode(edge.target);

      const sourceNodeInfo = getDetailsForNode(sourceNode, "source");
      const targetNodeInfo = getDetailsForNode(targetNode, "destination");

      if (sourceNodeInfo === undefined || targetNodeInfo === undefined) {
        continue;
      }

      // FIXME: This logic feels brittle
      if (
        sourceNodeInfo.name === "snowflake_source" ||
        targetNodeInfo.name === "snowflake_destination" ||
        sourceNodeInfo.name === "mycelial_net_source" ||
        targetNodeInfo.name === "mycelial_net_destination"
      ) {
        section.push(sourceNodeInfo);
        section.push(targetNodeInfo);

        if (edge.data?.ids?.length > 0) {
          for (const id of edge.data.ids) {
            configs.push({ id: id, pipe: section });
            toDelete = toDelete.filter((ed) => ed !== id);
            // setEdgesToBeDeleted((eds) => eds.filter((ed) => ed !== id));
          }
        } else {
          new_configs.push({ id: 0, pipe: section, ui_id: edge.id });
        }
      } else {
        let topicName = getRandomString();

        if (edge.data?.myc_network_info?.topic !== undefined) {
          topicName = edge.data.myc_network_info.topic;
        }

        const baseURL = window.location.origin;
        const mycNetTargetNodeInfo = {
          name: "mycelial_net_destination",
          endpoint: `${baseURL}/ingestion`,
          token: token,
          topic: topicName,
        };

        const mycNetSourceNodeInfo = {
          name: "mycelial_net_source",
          endpoint: `${baseURL}/ingestion`,
          token: token,
          topic: topicName,
        };

        section.push(sourceNodeInfo);
        section.push(mycNetTargetNodeInfo);
        if (edge.data?.ids?.length > 0) {
          configs.push({ id: edge.data.ids[0], pipe: section });
          toDelete = toDelete.filter((ed) => ed !== edge.data.ids[0]);
        } else {
          new_configs.push({ id: 0, pipe: section, ui_id: edge.id });
        }

        section = [];
        section.push(mycNetSourceNodeInfo);
        section.push(targetNodeInfo);
        if (edge.data?.ids?.length > 1) {
          configs.push({ id: edge.data.ids[1], pipe: section });
          toDelete = toDelete.filter((ed) => ed !== edge.data.ids[1]);
        } else {
          new_configs.push({ id: 0, pipe: section, ui_id: edge.id });
        }
      }
    }

    const payload = {
      configs: configs,
    };

    if (configs.length > 0) {
      try {
        const response = await fetch("/api/pipe/configs", {
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
    }

    for (const config of new_configs) {
      const new_payload = {
        configs: [config],
      };
      try {
        // todo: execute the fetches in parallel
        const response = await fetch("/api/pipe/configs", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Authorization": "Bearer " + btoa(token),
          },
          body: JSON.stringify(new_payload),
        });

        const result = await response.json();

        rf.setEdges((eds) => {
          return eds.map((ed) => {
            return {
              data: {
                id: result[0].id,
              },
              ...ed,
            };
          });
        });
      } catch (error) {
        console.error("Error:", error);
      }
    }

    for (const key in toDelete) {
      try {
        const response = await fetch(
          `/api/pipe/configs/${edgesToBeDeleted[key]}`,
          {
            method: "DELETE",
            headers: {
              "Content-Type": "application/json",
              "X-Authorization": "Bearer " + btoa(token),
            },
          }
        );

        await response.json();
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
      const {client: client, source: source, destination: destination} = JSON.parse(event.dataTransfer.getData("application/json"));

      // check if the dropped element is valid
      if (typeof client === "undefined" || !client) {
        return;
      }

      const position = reactFlowInstance.project({
        x: event.clientX - reactFlowBounds.left,
        y: event.clientY - reactFlowBounds.top,
      });

      let newNode: Node;
      if (source) {
        const type = `${source.type}Source`;
        newNode = {
          id: getId(),
          type,
          position,
          data: { label: `${type} node`, client: client.id, ...source },
        };
      } else if (destination) {
        const type = `${destination.type}Destination`;
        newNode = {
          id: getId(),
          type,
          position,
          data: { label: `${type} node`, client: client.id, ...destination },
        };
      } else {
        console.error("either source or destination should be set on the drag element")
        return;
      }

      setNodes((nds) => nds.concat(newNode));
    },
    [reactFlowInstance, setNodes]
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
                  {/* <button className="text-blue-300 bg-blue-50 rounded p-2 drop-shadow-md hover:bg-blue-100" onClick={() => removeMycelialNetworkNodes()}>remove myc net nodes</button> */}
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
