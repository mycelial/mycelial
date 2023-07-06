'use client'

import { useCallback, useState, useRef, DOMElement } from 'react';
import ReactFlow, {
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
} from 'reactflow';
import CustomNode from './CustomNode';

import styles from './Flow.module.css';
// import DatabaseNode from './DatabaseNode';

import { DatabaseSourceNode, DatabaseSinkNode, SourceTableNode, TargetTableNode, ViewNode } from '@/components/nodes';

import { Grid } from '@/components/layout';

import { UserButton } from '@/components/UserButton';

import {
  createStyles,
  Navbar,
  TextInput,
  Code,
  UnstyledButton,
  Badge,
  Text,
  Group,
  ActionIcon,
  Tooltip,
  rem,
} from '@/components/core';

import {
  IconBulb,
  IconUser,
  IconCheckbox,
  IconSearch,
  IconPlus,
  IconSelector,
} from '@tabler/icons-react';

const useStyles = createStyles((theme) => ({
  navbar: {
    paddingTop: 0,
  },

  section: {
    marginLeft: `calc(${theme.spacing.md} * -1)`,
    marginRight: `calc(${theme.spacing.md} * -1)`,
    marginBottom: theme.spacing.md,

    '&:not(:last-of-type)': {
      borderBottom: `${rem(1)} solid ${
        theme.colorScheme === 'dark' ? theme.colors.dark[4] : theme.colors.gray[3]
      }`,
    },
  },

  searchCode: {
    fontWeight: 700,
    fontSize: rem(10),
    backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[7] : theme.colors.gray[0],
    border: `${rem(1)} solid ${
      theme.colorScheme === 'dark' ? theme.colors.dark[7] : theme.colors.gray[2]
    }`,
  },

  mainLinks: {
    paddingLeft: `calc(${theme.spacing.md} - ${theme.spacing.xs})`,
    paddingRight: `calc(${theme.spacing.md} - ${theme.spacing.xs})`,
    paddingBottom: theme.spacing.md,
  },

  mainLink: {
    display: 'flex',
    alignItems: 'center',
    width: '100%',
    fontSize: theme.fontSizes.xs,
    padding: `${rem(8)} ${theme.spacing.xs}`,
    borderRadius: theme.radius.sm,
    fontWeight: 500,
    color: theme.colorScheme === 'dark' ? theme.colors.dark[0] : theme.colors.gray[7],

    '&:hover': {
      backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[6] : theme.colors.gray[0],
      color: theme.colorScheme === 'dark' ? theme.white : theme.black,
    },
  },

  mainLinkInner: {
    display: 'flex',
    alignItems: 'center',
    flex: 1,
  },

  mainLinkIcon: {
    marginRight: theme.spacing.sm,
    color: theme.colorScheme === 'dark' ? theme.colors.dark[2] : theme.colors.gray[6],
  },

  mainLinkBadge: {
    padding: 0,
    width: rem(20),
    height: rem(20),
    pointerEvents: 'none',
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
    display: 'block',
    padding: `${rem(8)} ${theme.spacing.xs}`,
    textDecoration: 'none',
    borderRadius: theme.radius.sm,
    fontSize: theme.fontSizes.xs,
    color: theme.colorScheme === 'dark' ? theme.colors.dark[0] : theme.colors.gray[7],
    lineHeight: 1,
    fontWeight: 500,

    '&:hover': {
      backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[6] : theme.colors.gray[0],
      color: theme.colorScheme === 'dark' ? theme.white : theme.black,
    },
  },
}));

const links = [
  { icon: IconBulb, label: 'Activity', notifications: 3 },
  { icon: IconCheckbox, label: 'Tasks', notifications: 4 },
  // { icon: IconUser, label: 'Contacts' },
];

const collections = [
  { emoji: '👍', label: 'Database Source', nodeType: 'databaseSource' },
  { emoji: '👍', label: 'Database Sink', nodeType: 'databaseSink' },
  { emoji: '👍', label: 'Source Table', nodeType: 'sourceTable' },
  { emoji: '👍', label: 'Sink Table', nodeType: 'targetTable' },
  { emoji: '👍', label: 'View', nodeType: 'view' },
];

const initialNodes: Node[] = [
  /* {
    id: '1',
    type: 'input',
    data: { label: 'Node 1' },
    position: { x: 250, y: 5 },
  },
  {
    id: '2',
    data: { label: 'Node 2' },
    position: { x: 100, y: 100 },
  },
  {
    id: '3',
    data: { label: 'Node 3' },
    position: { x: 400, y: 100 },
  },
  {
    id: '4',
    data: { label: 'Node 4' },
    position: { x: 400, y: 200 },
    type: 'custom',
    className: styles.customNode,
  },
  {
    id: '5',
    data: { label: 'Database Node' },
    position: { x: 500, y: 400 },
    type: 'database',
    className: styles.customNode,
  },
  {
    id: '6',
    data: { label: 'Table Node' },
    position: { x: 500, y: 400 },
    type: 'table',
    className: styles.customNode,
  },
  {
    id: '7',
    data: { label: 'View Node' },
    position: { x: 500, y: 400 },
    type: 'view',
    className: styles.customNode,
  }, */
];

const initialEdges: Edge[] = [
  /* { id: 'e1-2', source: '1', target: '2' },
  { id: 'e1-3', source: '1', target: '3' }, */
];

const nodeTypes = {
  custom: CustomNode,
  databaseSource: DatabaseSourceNode,
  databaseSink: DatabaseSinkNode,
  sourceTable: SourceTableNode,
  targetTable: TargetTableNode,
  view: ViewNode,
};

const defaultEdgeOptions = {
  animated: true,
  type: 'smoothstep',
};

let id = 0;
const getId = () => `dndnode_${id++}`;

// const onInit = (reactFlowInstance: any) => console.log('flow loaded:', reactFlowInstance);

function Sidebar() {
  const onDragStart = (event: any, nodeType: string) => {
    event.dataTransfer.setData('application/reactflow', nodeType);
    event.dataTransfer.effectAllowed = 'move';
  };

  return (
    <aside>
      <div className="description">You can drag these nodes to the main pane.</div>
      <div className="dndnode" onDragStart={(event) => onDragStart(event, 'databaseSource')} draggable>
        Database Source Node
      </div>
      <div className="dndnode" onDragStart={(event) => onDragStart(event, 'databaseSink')} draggable>
        Database Sink Node
      </div>
      <div className="dndnode" onDragStart={(event) => onDragStart(event, 'sourceTable')} draggable>
        Pick Tables Node
      </div>
      <div className="dndnode" onDragStart={(event) => onDragStart(event, 'targetTable')} draggable>
        Target Table Node
      </div>
      <div className="dndnode" onDragStart={(event) => onDragStart(event, 'view')} draggable>
        View Node
      </div>
    </aside>
  );
}

function NavbarSearch() {
  const { classes } = useStyles();

  const onDragStart = (event: any, nodeType: string) => {
    event.dataTransfer.setData('application/reactflow', nodeType);
    event.dataTransfer.effectAllowed = 'move';
  };

  const mainLinks = links.map((link) => (
    <UnstyledButton key={link.label} className={classes.mainLink}>
      <div className={classes.mainLinkInner}>
        <link.icon size={20} className={classes.mainLinkIcon} stroke={1.5} />
        <span>{link.label}</span>
      </div>
      {link.notifications && (
        <Badge size="sm" variant="filled" className={classes.mainLinkBadge}>
          {link.notifications}
        </Badge>
      )}
    </UnstyledButton>
  ));

  const collectionLinks = collections.map((collection) => (
    <div className={classes.collectionLink} key={collection.label} onDragStart={(event) => onDragStart(event, collection.nodeType)} draggable>
    {collection.label}
    </div>
  ));

  return (
    <Navbar height={700} width={{ sm: 250 }} p="md" className={classes.navbar}>
      <Navbar.Section className={classes.section}>
        <UserButton
          image="https://i.imgur.com/fGxgcDF.png"
          name="Petya Batkovich"
          email="Business Analyst"
          icon={<IconSelector size="0.9rem" stroke={1.5} />}
        />
      </Navbar.Section>

      <TextInput
        placeholder="Search"
        size="xs"
        icon={<IconSearch size="0.8rem" stroke={1.5} />}
        rightSectionWidth={70}
        rightSection={<Code className={classes.searchCode}>Ctrl + K</Code>}
        styles={{ rightSection: { pointerEvents: 'none' } }}
        mb="sm"
      />

      <Navbar.Section className={classes.section}>
        <div className={classes.mainLinks}>{mainLinks}</div>
      </Navbar.Section>

      <Navbar.Section className={classes.section}>
        <Group className={classes.collectionsHeader} position="apart">
          <Text size="xs" weight={500} color="dimmed">
            Available Nodes
          </Text>
          <Tooltip label="Create collection" withArrow position="right">
            <ActionIcon variant="default" size={18}>
              <IconPlus size="0.8rem" stroke={1.5} />
            </ActionIcon>
          </Tooltip>
        </Group>
        <div className={classes.collections}>{collectionLinks}</div>
      </Navbar.Section>
    </Navbar>
  );
}

function Flow() {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const onConnect = useCallback((params: Connection | Edge) => setEdges((eds) => addEdge(params, eds)), []);

  console.log(nodes);
  console.log(edges);

  const [reactFlowInstance, setReactFlowInstance] = useState<ReactFlowInstance | null>(null);

  const onDragOver = useCallback((event: any) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
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
      const type = event.dataTransfer.getData('application/reactflow');

      // check if the dropped element is valid
      if (typeof type === 'undefined' || !type) {
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
    [reactFlowInstance]
  );

  const minimapStyle = {
    height: 120,
  };

  return (
    <ReactFlowProvider>
      <Grid>
        <Grid.Col span={2}><NavbarSearch /></Grid.Col>
        <Grid.Col span={10}><div className={styles.flow} ref={reactFlowWrapper}>
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
            <MiniMap style={minimapStyle} zoomable pannable />
            <Controls />
            <Background color="#aaa" gap={16} />
          </ReactFlow>
        </div></Grid.Col>
      </Grid>

    
    </ReactFlowProvider>
  );
}

export default Flow;