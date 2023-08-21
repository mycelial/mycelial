import { useMemo, memo, FC, useEffect, useContext, useCallback } from "react";
import {
  Handle,
  Position,
  NodeProps,
  NodeResizer,
  useNodeId,
  useReactFlow,
  getConnectedEdges,
} from "reactflow";

import { Select, MultiSelect, TextArea, TextInput } from "@/components/inputs";
import { Grid } from "@/components/layout";

import {
  createStyles,
  rem,
  Flex
} from "@/components/core";

import { ClientContext } from "./context/clientContext";
import { ClientContextType } from "./@types/client";


const useStyles = createStyles((theme) => ({
  customNode: {
    background: theme.colors.night[1],
    borderRadius: rem(5),

  },
  deleteNodeButton: {
    width: "auto",
    height: rem(15),
    // TODO: fix margins on button
    marginLeft: rem(10),
    background: "none",
    border: `1px solid ${theme.colors.toadstool[2]}`,
    borderRadius: rem(10),
    color: theme.colors.toadstool[2],
  },
  // TODO: Implement border highlight when node is selected 
  nodeTitle: {
    background: theme.colors.forest[1],
    padding: rem(5),
    borderTopLeftRadius: rem(5),
    borderTopRightRadius: rem(5),
  },
}));

  

const SqliteSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const { classes } = useStyles();
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      databasePath: data.path ? data.path : "/tmp/test.sqlite",
      tables: data.tables ? data.tables : "*",
      client: data.client ? data.client : "-",
    };
  }, []);

  const handleChange = useCallback((name: string, value: string) => {
    instance.setNodes((nodes) =>
      nodes.map((node) => {
        if (node.id === id) {
          node.data = {
            ...node.data,
            [name]: value,
          };
        }

        return node;
      })
    );
  }, []);

  useEffect(() => {
    handleChange("path", initialValues.databasePath);
    handleChange("tables", initialValues.tables);
    handleChange("client", initialValues.client);
  }, []);



  const removeNode = useCallback((id: string) => {
    const node = instance.getNode(id);
    if (node === undefined) {
      return;
    }
    instance.deleteElements({
      edges: getConnectedEdges([node], []),
      nodes: [node],
    });
  }, []);

  return (
    <div className={classes.customNode}>
      <div className="">
        <h2 className="">SQLite Source</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
        </button>
        <TextInput
          name="sqliteDatabasePath"
          label="Sqlite Database Path"
          placeholder={initialValues.databasePath}
          defaultValue={initialValues.databasePath}
          onChange={(event) => handleChange("path", event.currentTarget.value)}
        />
        <TextInput
          name="sqliteQuery"
          label="Tables"
          placeholder={initialValues.tables}
          defaultValue={initialValues.tables}
          onChange={(event) =>
            handleChange("tables", event.currentTarget.value)
          }
        />
        <Select
          name="client"
          label="Client"
          placeholder="Pick one"
          defaultValue={initialValues.client}
          options={(clients || []).map((c) => c.id)}
          onChange={(value) => {
            handleChange("client", value || "");
          }}
        />
        <Handle type="source" position={Position.Right} id={id} />
      </div>
    </div>
  );
});

const SqliteDestinationNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { classes } = useStyles();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      databasePath: data.path ? data.path : "/tmp/test_dst.sqlite",
      client: data.client ? data.client : "-",
    };
  }, []);

  const handleChange = useCallback((name: string, value: string) => {
    instance.setNodes((nodes) =>
      nodes.map((node) => {
        if (node.id === id) {
          node.data = {
            ...node.data,
            [name]: value,
          };
        }

        return node;
      })
    );
  }, []);

  const removeNode = useCallback((id: string) => {
    let node = instance.getNode(id);
    if (node === undefined) {
      return;
    }
    let edges = getConnectedEdges([node], []);
    instance.deleteElements({ edges, nodes: [node] });
  }, []);

  useEffect(() => {
    handleChange("path", initialValues.databasePath);
    handleChange("client", initialValues.client);
  }, []);



  return (
    <div className={classes.customNode}>
      <Flex
        gap="md"
        justify="center"
        align="left"
        direction="column"
        wrap="nowrap"
      >
        <Flex className={classes.nodeTitle} justify="space-between" direction="row">
          <h2>SQLite Destination</h2>

          <button
            onClick={() => {
              if (confirm("Are you sure you want to delete this node?")) {
                removeNode(id);
              }
            }}
            type="button"
            className={classes.deleteNodeButton}
            title="delete"
          >
            X
          </button>
        </Flex>

        <TextInput
          
          name="sqliteDatabasePath"
          label="Sqlite Database Path"
          placeholder={initialValues.databasePath}
          defaultValue={initialValues.databasePath}
          onChange={(event) => handleChange("path", event.currentTarget.value)}
        />
        <Select
          name="client"
          label="Client"
          placeholder="Pick one"
          defaultValue={initialValues.client}
          options={(clients || []).map((c) => c.id)}
          onChange={(value) => {
            handleChange("client", value || "");
          }}
        />
        <Handle type="target" position={Position.Left} id={id} />
      </Flex>
    </div>
  );
});

const MycelialNetworkNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { classes } = useStyles();
  let initialValues = useMemo(() => {
    return {
      endpoint: data.endpoint
        ? data.endpoint
        : "http://localhost:8080/ingestion",
      token: data.token ? data.token : "...",
      topic: data.topic ? data.topic : "...",
    };
  }, []);

  const removeNode = useCallback((id: string) => {
    let node = instance.getNode(id);
    if (node === undefined) {
      return;
    }
    let edges = getConnectedEdges([node], []);
    instance.deleteElements({ edges, nodes: [node] });
  }, []);

  const handleChange = useCallback((name: string, value: string) => {
    instance.setNodes((nodes) =>
      nodes.map((node) => {
        if (node.id === id) {
          node.data = {
            ...node.data,
            [name]: value,
          };
        }

        return node;
      })
    );
  }, []);

  useEffect(() => {
    handleChange("endpoint", initialValues.endpoint);
    handleChange("token", initialValues.token);
    handleChange("topic", initialValues.topic);
  }, []);


  // if (selected) {
  //   classNames = classNames + `${classes.selected}`;
  // }
  return (
    <div className={classes.customNode}>
      <div className="">
        <Handle type="target" position={Position.Left} id={id} />
        <Handle type="source" position={Position.Right} id={id} />
        <h2 className="">Mycelial Network</h2>

        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className=""
          title="delete"
        >
          <XMarkIcon className="" aria-hidden="true" />
        </button>
        <TextInput
          name="endpoint"
          label="Endpoint"
          placeholder={initialValues.endpoint}
          defaultValue={initialValues.endpoint}
          onChange={(event) =>
            handleChange("endpoint", event.currentTarget.value)
          }
        />
        <TextInput
          name="token"
          label="Token"
          placeholder={initialValues.token}
          defaultValue={initialValues.token}
          onChange={(event) => handleChange("token", event.currentTarget.value)}
        />
        <TextInput
          name="topic"
          label="Topic"
          placeholder={initialValues.topic}
          defaultValue={initialValues.topic}
          onChange={(event) => handleChange("topic", event.currentTarget.value)}
        />
      </div>
    </div>
  );
});

const SnowflakeDestinationNode: FC<NodeProps> = memo(
  ({ id, data, selected }) => {
    const instance = useReactFlow();
    const { classes } = useStyles();
    let initialValues = useMemo(() => {
      return {
        username: data.username ? data.username : "SVCMYCELIAL",
        password: data.password ? data.password : "",
        role: data.role ? data.role : "MYCELIAL",
        account_identifier: data.account_identifier
          ? data.account_identifier
          : "UW17194-STREAMLITPR",
        warehouse: data.warehouse ? data.warehouse : "MYCELIAL",
        database: data.database ? data.database : "MYCELIAL",
        schema: data.schema ? data.schema : "PIPE",
        table: data.table ? data.table : "TEST_DESTINATION",
      };
    }, []);

    const handleChange = useCallback((name: string, value: string) => {
      instance.setNodes((nodes) =>
        nodes.map((node) => {
          if (node.id === id) {
            node.data = {
              ...node.data,
              [name]: value,
            };
          }

          return node;
        })
      );
    }, []);

    useEffect(() => {
      handleChange("username", initialValues.username);
      handleChange("password", initialValues.password);
      handleChange("role", initialValues.role);
      handleChange("account_identifier", initialValues.account_identifier);
      handleChange("warehouse", initialValues.warehouse);
      handleChange("database", initialValues.database);
      handleChange("schema", initialValues.schema);
      handleChange("table", initialValues.table);
    }, []);

    // if (selected) {
    //   classNames = classNames + `${styles.selected}`;
    // }

    const removeNode = useCallback((id: string) => {
      let node = instance.getNode(id);
      if (node === undefined) {
        return;
      }
      let edges = getConnectedEdges([node], []);
      instance.deleteElements({ edges, nodes: [node] });
    }, []);

    return (
      <div className={classes.customNode}>
        <div className="">
          <h2 className="">Snowflake Destination</h2>
          <button
            onClick={() => {
              if (confirm("Are you sure you want to delete this node?")) {
                removeNode(id);
              }
            }}
            type="button"
            
            title="delete"
          >
            {/* <XMarkIcon className="" aria-hidden="true" /> */}
            X 
          </button>
          <TextInput
            name="username"
            label="Username"
            placeholder={initialValues.username}
            defaultValue={initialValues.username}
            onChange={(event) =>
              handleChange("username", event.currentTarget.value)
            }
          />
          <TextInput
            name="password"
            label="Password"
            placeholder={initialValues.password}
            defaultValue={initialValues.password}
            onChange={(event) =>
              handleChange("password", event.currentTarget.value)
            }
          />
          <TextInput
            name="role"
            label="Role"
            placeholder={initialValues.role}
            defaultValue={initialValues.role}
            onChange={(event) =>
              handleChange("role", event.currentTarget.value)
            }
          />
          <TextInput
            name="account_identifier"
            label="Account Identifier"
            placeholder={initialValues.account_identifier}
            defaultValue={initialValues.account_identifier}
            onChange={(event) =>
              handleChange("account_identifier", event.currentTarget.value)
            }
          />
          <TextInput
            name="warehouse"
            label="Warehouse"
            placeholder={initialValues.warehouse}
            defaultValue={initialValues.warehouse}
            onChange={(event) =>
              handleChange("warehouse", event.currentTarget.value)
            }
          />
          <TextInput
            name="database"
            label="Database"
            placeholder={initialValues.database}
            defaultValue={initialValues.database}
            onChange={(event) =>
              handleChange("database", event.currentTarget.value)
            }
          />
          <TextInput
            name="schema"
            label="Schema"
            placeholder={initialValues.schema}
            defaultValue={initialValues.schema}
            onChange={(event) =>
              handleChange("schema", event.currentTarget.value)
            }
          />
          <TextInput
            name="table"
            label="Table"
            placeholder={initialValues.table}
            defaultValue={initialValues.table}
            onChange={(event) =>
              handleChange("table", event.currentTarget.value)
            }
          />
          <Handle type="target" position={Position.Left} id={id} />
        </div>
      </div>
    );
  }
);

const SnowflakeSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { classes } = useStyles();
  let initialValues = useMemo(() => {
    return {
      username: data.username ? data.username : "SVCMYCELIAL",
      password: data.password ? data.password : "",
      role: data.role ? data.role : "MYCELIAL",
      account_identifier: data.account_identifier
        ? data.account_identifier
        : "UW17194-STREAMLITPR",
      warehouse: data.warehouse ? data.warehouse : "MYCELIAL",
      database: data.database ? data.database : "MYCELIAL",
      schema: data.schema ? data.schema : "GITHUB",
      query: data.query ? data.query : "SELECT * FROM COMMIT",
    };
  }, []);

  const handleChange = useCallback((name: string, value: string) => {
    instance.setNodes((nodes) =>
      nodes.map((node) => {
        if (node.id === id) {
          node.data = {
            ...node.data,
            [name]: value,
          };
        }

        return node;
      })
    );
  }, []);

  useEffect(() => {
    handleChange("username", initialValues.username);
    handleChange("password", initialValues.password);
    handleChange("role", initialValues.role);
    handleChange("account_identifier", initialValues.account_identifier);
    handleChange("warehouse", initialValues.warehouse);
    handleChange("database", initialValues.database);
    handleChange("schema", initialValues.schema);
    handleChange("query", initialValues.query);
  }, []);

  const removeNode = useCallback((id: string) => {
    let node = instance.getNode(id);
    if (node === undefined) {
      return;
    }
    let edges = getConnectedEdges([node], []);
    instance.deleteElements({ edges, nodes: [node] });
  }, []);

  return (
    <div className={classes.customNode}>
      <div className="">
        <h2 className="">Snowflake Source</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className=""
          title="delete"
        >
          <XMarkIcon className="" aria-hidden="true" />
        </button>
        <TextInput
          name="nodrag"
          label="Username"
          placeholder={initialValues.username}
          defaultValue={initialValues.username}
          onChange={(event) =>
            handleChange("username", event.currentTarget.value)
          }
        />
        <TextInput
          name="password"
          label="Password"
          placeholder={initialValues.password}
          defaultValue={initialValues.password}
          onChange={(event) =>
            handleChange("password", event.currentTarget.value)
          }
        />
        <TextInput
          name="role"
          label="Role"
          placeholder={initialValues.role}
          defaultValue={initialValues.role}
          onChange={(event) => handleChange("role", event.currentTarget.value)}
        />
        <TextInput
          name="account_identifier"
          label="Account Identifier"
          placeholder={initialValues.account_identifier}
          defaultValue={initialValues.account_identifier}
          onChange={(event) =>
            handleChange("account_identifier", event.currentTarget.value)
          }
        />
        <TextInput
          name="warehouse"
          label="Warehouse"
          placeholder={initialValues.warehouse}
          defaultValue={initialValues.warehouse}
          onChange={(event) =>
            handleChange("warehouse", event.currentTarget.value)
          }
        />
        <TextInput
          name="database"
          label="Database"
          placeholder={initialValues.database}
          defaultValue={initialValues.database}
          onChange={(event) =>
            handleChange("database", event.currentTarget.value)
          }
        />
        <TextInput
          name="schema"
          label="Schema"
          placeholder={initialValues.schema}
          defaultValue={initialValues.schema}
          onChange={(event) =>
            handleChange("schema", event.currentTarget.value)
          }
        />
        <TextInput
          name="query"
          label="Query"
          placeholder={initialValues.query}
          defaultValue={initialValues.query}
          onChange={(event) => handleChange("query", event.currentTarget.value)}
        />
        <Handle type="source" position={Position.Right} id={id} />
      </div>
    </div>
  );
});

const KafkaSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;
  const { classes } = useStyles();
  let initialValues = useMemo(() => {
    return {
      brokers: data.brokers ? data.brokers : "localhost:9092",
      group_id: data.group_id ? data.group_id : "group_id",
      topics: data.topics ? data.topics : "topic-0",
      client: data.client ? data.client : "-",
    };
  }, []);

  const handleChange = useCallback((name: string, value: string) => {
    instance.setNodes((nodes) =>
      nodes.map((node) => {
        if (node.id === id) {
          node.data = {
            ...node.data,
            [name]: value,
          };
        }

        return node;
      })
    );
  }, []);

  const removeNode = useCallback((id: string) => {
    let node = instance.getNode(id);
    if (node === undefined) {
      return;
    }
    let edges = getConnectedEdges([node], []);
    instance.deleteElements({ edges, nodes: [node] });
  }, []);

  useEffect(() => {
    handleChange("brokers", initialValues.brokers);
    handleChange("group_id", initialValues.group_id);
    handleChange("topics", initialValues.topics);
    handleChange("client", initialValues.client);
  }, []);


  return (
    <div className={classes.customNode}>
      <div className="">
        <h2 className="">Kafka Source</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className=""
          title="delete"
        >
          <XMarkIcon className="" aria-hidden="true" />
        </button>
        <TextInput
          name="brokers"
          label="Brokers"
          placeholder={initialValues.brokers}
          defaultValue={initialValues.brokers}
          onChange={(event) =>
            handleChange("brokers", event.currentTarget.value)
          }
        />
        <TextInput
          name="groupId"
          label="GroupId"
          placeholder={initialValues.group_id}
          defaultValue={initialValues.group_id}
          onChange={(event) =>
            handleChange("group_id", event.currentTarget.value)
          }
        />
        <TextInput
          name="topics"
          label="Topics"
          placeholder={initialValues.topics}
          defaultValue={initialValues.topics}
          onChange={(event) =>
            handleChange("topics", event.currentTarget.value)
          }
        />
        <Select
          name="client"
          label="Client"
          placeholder="Pick one"
          defaultValue={initialValues.client}
          options={(clients || []).map((c) => c.id)}
          onChange={(value) => handleChange("client", value || "")}
        />
        <Handle type="source" position={Position.Right} id={id} />
      </div>
    </div>
  );
});

export {
  SqliteSourceNode,
  SqliteDestinationNode,
  MycelialNetworkNode,
  KafkaSourceNode,
  SnowflakeSourceNode,
  SnowflakeDestinationNode,
};
