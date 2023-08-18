"use client";

import {
  useCallback,
  useContext,
  useState,
  useRef,
  DOMElement,
  useEffect,
  createContext,
} from "react";

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

import { Grid, Container, Anchor, Group } from "@/components/layout";

import { UserButton } from "@/components/UserButton";


import {
  createStyles,
  Navbar,
  TextInput,
  Code,
  UnstyledButton,
  Badge,
  Text,
  // Group,
  ActionIcon,
  Tooltip,
  rem,
  Button, 
  Box,
  Image
} from "@/components/core";

import {
  IconBulb,
  IconUser,
  IconCheckbox,
  IconSearch,
  IconPlus,
  IconSelector,
} from "@tabler/icons-react";
import ClientProvider, { ClientContext } from "../context/clientContext";
import { ClientContextType } from "../@types/client";

const useStyles = createStyles((theme) => ({
  navbar: {
    paddingTop: 100,
    background: theme.colors.night[1], 
    borderRight: `${theme.colors.night[2]} ${rem(1)} solid`,
  },
  section: {
    marginLeft: `calc(${theme.spacing.md} * -1)`,
    marginRight: `calc(${theme.spacing.md} * -1)`,
    marginBottom: theme.spacing.md,

    "&:not(:last-of-type)": {
      borderBottom: `${rem(1)} solid ${theme.colors.night[2]}`,
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
  
  },

  mainLinkInner: {
    display: "flex",
    alignItems: "center",
    flex: 1,
  },

  mainLinkIcon: {
    marginRight: theme.spacing.sm,
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
    color: theme.colors.moss[1],
  },

  collectionLink: {
    display: "block",
    padding: `${rem(8)} ${theme.spacing.xs}`,
    textDecoration: "none",
    borderRadius: theme.radius.sm,
    fontSize: theme.fontSizes.xs,
   
    lineHeight: 1,
    fontWeight: 500,
    cursor: "grab",
    "&:hover": {
      borderColor: theme.colors.stem[0],
      borderWidth: 1,
      backgroundColor: theme.colors.forest[0],
    },

  },

  buttonHoverStyle: {
    "&:hover": {
      borderColor: theme.colors.stem[0],
      borderWidth: 1,
      backgroundColor: theme.colors.forest[0],

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

  const onDragStart = (event: any, nodeType: string) => {
    event.dataTransfer.setData("application/reactflow", nodeType);
    event.dataTransfer.effectAllowed = "move";
  };

  const collectionLinks = collections.map((collection) => (
    <div
      className={classes.collectionLink}
      key={collection.label}
      onDragStart={(event) => onDragStart(event, collection.nodeType)}
      draggable
    >
      {collection.label}
    </div>
  ));

  return (
    <Navbar
      height="100vh"
      width={{ sm: 200 }}
      p="md"
      
      className={classes.navbar}
    >
      <Image
        mb={16}
        src="/mycelial.svg"
        alt="Mycelial Logo"
        maw={240}
        mx="auto"

      />
      <Navbar.Section className={classes.section}>
        <Group className={classes.collectionsHeader} position="apart">
          <Text size="xs" weight={500} > 
            Data Sources
          </Text>
        </Group>
        <div className={classes.collections}>{collectionLinks}</div>
        <div>
          <Box style={{ padding: "1rem" }}>
            <Button
              fullWidth
              leftIcon={<IconDatabase size="1rem" />}
              onClick={props.onSave}
              className={classes.buttonHoverStyle}
              bg="forest.0"
              c="stem.0"
            >
              Publish
            </Button>
          </Box>
        </div>
      </Navbar.Section>
      <Navbar.Section className={classes.section}>
        <Group className={classes.collectionsHeader} position="apart">
          <Text size="xs" weight={500}>
            Available Clients
          </Text>
        </Group>
        <div className={classes.collections}>
          {(clients || []).map((client) => (
            <div className={classes.collectionLink} key={client.id}>
              {client.id}
            </div>
          ))}
        </div>
      </Navbar.Section>
    </Navbar>
  );
}

async function getStartingUI(token: string) {
  try {
    const response = await fetch("/api/ui-metadata", {
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

function Flow() {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const onConnect = useCallback(
    (params: Connection | Edge) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  const { token } = (useContext(ClientContext) as ClientContextType) || {};

  const setInitialElements = useCallback(async () => {
    let ui_metadata = await getStartingUI(token);

    ui_metadata?.ui_metadata.nodes.forEach((node: any) => {
      setNodes((nds) => nds.concat(node));
    });

    ui_metadata?.ui_metadata.edges.forEach((edge: any) => {
      setEdges((eds) => eds.concat(edge));
    });
  }, [setEdges, setNodes, token]);

  useEffect(() => {
    setInitialElements();
  }, [setInitialElements]);

  const [reactFlowInstance, setReactFlowInstance] =
    useState<ReactFlowInstance | null>(null);

  const getDetailsForNode = useCallback((node: Node | undefined, kind: String) => {
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
  }, []);

  const handleSave = useCallback(async () => {
    let configs = [];

    if (reactFlowInstance === null) {
      return;
    }

    const rf = reactFlowInstance;

    let configId = 0;

    for (const edge of edges) {
      ++configId;

      const section = [];

      const sourceNode = rf.getNode(edge.source);
      const targetNode = rf.getNode(edge.target);

      const sourceNodeInfo = getDetailsForNode(sourceNode, "source");
      const targetNodeInfo = getDetailsForNode(targetNode, "destination");

      section.push(sourceNodeInfo);
      section.push(targetNodeInfo);

      configs.push({ id: configId, pipe: section });
    }

    const payload = {
      configs: configs,
      ui_metadata: rf.toObject(),
    };

    try {
      const response = await fetch("/pipe/configs", {
        method: "POST", // or 'PUT'
        headers: {
          "Content-Type": "application/json",
          "X-Authorization": "Bearer " + btoa(token),
        },
        body: JSON.stringify(payload),
      });

      const result = await response.json();
      console.log("Success:", result);
    } catch (error) {
      console.error("Error:", error);
    }
  }, [edges, reactFlowInstance, token, getDetailsForNode]);

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
      const type = event.dataTransfer.getData("application/reactflow");

      // check if the dropped element is valid
      if (typeof type === "undefined" || !type) {
        return;
      }

      const position = reactFlowInstance.project({
        x: event.clientX - reactFlowBounds.left,
        y: event.clientY - reactFlowBounds.top,
      });
      const newNode = {
        id: getId(),
        type,
        position,
        data: { label: `${type} node` },
      };

      setNodes((nds) => nds.concat(newNode));
    },
    [reactFlowInstance, setNodes]
  );

  const minimapStyle = {
    height: 120,
  };

  useEffect(() => {
    if (reactFlowInstance === null) {
      return;
    }
  }, [reactFlowInstance]);



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
                onEdgesChange={onEdgesChange}
                onConnect={onConnect}
                nodeTypes={nodeTypes}
                defaultEdgeOptions={defaultEdgeOptions}
                connectionLineType={ConnectionLineType.SmoothStep}
                onInit={setReactFlowInstance}
                onDrop={onDrop}
                onDragOver={onDragOver}
                fitView
                snapToGrid={true}
                
              >
                <Controls />
              </ReactFlow>
            </div>
          </Grid.Col>
        </Grid>
      </ClientProvider>
    </ReactFlowProvider>
  );
}

export default Flow;
