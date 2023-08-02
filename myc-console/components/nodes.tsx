import { memo, FC, useEffect, useContext, useCallback } from "react";
import {
  Handle,
  Position,
  NodeProps,
  NodeResizer,
  useNodeId,
  useReactFlow,
} from "reactflow";

import { Select, MultiSelect, TextArea, TextInput } from "@/components/inputs";

import styles from "@/components/Flow/Flow.module.css";
import { ClientContext } from "./context/clientContext";
import { ClientContextType } from "./@types/client";

const SqliteSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = {
    databasePath: data.path ? data.path : "/tmp/test.sqlite",
    tables: data.tables? data.tables: "*",
    client: data.client ? data.client : "",
  };

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
  }, [id]);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">SQLite Source</h2>
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
          onChange={(event) => handleChange("tables", event.currentTarget.value)}
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
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = {
    databasePath: data.path ? data.path : "/tmp/test_dst.sqlite",
    client: data.client ? data.client : "",
  };

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
  }, [id]);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">SQLite Destination</h2>
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
      </div>
    </div>
  );
})

const MycelialNetworkNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = {
    endpoint: data.endpoint ? data.endpoint : "http://localhost:8080/ingestion",
    token: data.token ? data.token : "...",
  };

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
  }, [id]);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }
  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
      <Handle type="target" position={Position.Left} id={id} />
      <h2 className="text-slate-400 font-normal">Mycelial Network</h2>
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
    </div>
    </div>
  );
});

const SnowflakeDestinationNode: FC<NodeProps> = memo(
  ({ id, data, selected }) => {
    const instance = useReactFlow();

    let initialValues = {
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
    }, [id]);

    let classNames = `${styles.customNode} `;
    if (selected) {
      classNames = classNames + `${styles.selected}`;
    }

    return (
      <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">Snowflake Sink</h2>
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
          name="table"
          label="Table"
          placeholder={initialValues.table}
          defaultValue={initialValues.table}
          onChange={(event) => handleChange("table", event.currentTarget.value)}
        />
        <Handle type="target" position={Position.Left} id={id} />
      </div>
      </div>
    );
  }
);

const SnowflakeSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();

  let initialValues = {
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
  }, [id]);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
      <h2 className="text-slate-400 font-normal">Snowflake Source</h2>
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
        onChange={(event) => handleChange("schema", event.currentTarget.value)}
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

  let initialValues = {
    brokers: data.brokers ? data.brokers : "localhost:9092",
    group_id: data.group_id ? data.group_id : "group_id",
    topics: data.topics ? data.topics : "topic-0",
    client: data.client ? data.client : " ",
  };

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
    handleChange("brokers", initialValues.brokers);
    handleChange("group_id", initialValues.group_id);
    handleChange("topics", initialValues.topics);
    handleChange("client", initialValues.client);
  }, [id]);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }
  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
      <h2 className="text-slate-400 font-normal">Kafka Source</h2>
      <TextInput
        name="brokers"
        label="Brokers"
        placeholder={initialValues.brokers}
        defaultValue={initialValues.brokers}
        onChange={(event) => handleChange("brokers", event.currentTarget.value)}
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
        onChange={(event) => handleChange("topics", event.currentTarget.value)}
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

const DatabaseSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();

  useEffect(() => {
    instance.setNodes((nodes) =>
      nodes.map((node) => {
        if (node.id === id) {
          node.data = {
            ...node.data,
            customId: id,
          };
        }

        return node;
      })
    );
  }, [id]);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }
  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
      <h2 className="text-slate-400 font-normal">Database Source</h2>
      <Select
        name="database_source"
        label="Database Source"
        options={["Sqlite", "PostgreSQL", "Snowflake"]}
        onChange={(event) => {}}
      />
      <MultiSelect
        className="nodrag"
        label="Node labels"
        placeholder="Pick multiple"
        searchable
        nothingFound="No options"
        data={["device:drone", "size:xs", "sensor:temperature"]}
        withinPortal
      />
      <Handle type="source" position={Position.Right} id={id} />
    </div>
    </div>
  );
});

const DatabaseSinkNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }
  return (
    <div className={classNames}>
      <h2 className="text-slate-400 font-normal">Database Sink</h2>
      <Handle type="target" position={Position.Left} id={id} />
      <Select
        name="database_sink"
        label="Database Sink"
        options={["Snowflake"]}
        onChange={() => {}}
      />
    </div>
  );
});

const SourceTableNode: FC<NodeProps> = memo((props) => {
  const { id, data } = props;

  let classNames = `${styles.customNode} `;
  if (props.selected) {
    classNames = classNames + `${styles.selected}`;
  }
  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
      <Handle type="target" position={Position.Left} id={id} />
      <Select
        name="table"
        label="Table"
        options={["users", "orders", "orders_pending", "orders_complete"]}
        onChange={() => {}}
      />
      <Handle type="source" position={Position.Right} id={id} />
    </div>
    </div>
  );
});

const TargetTableNode: FC<NodeProps> = memo(({ selected, id, data }) => {
  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }
  return (
    <div className={classNames}>
      <div className="grid grid-cols-1 gap-x-6 gap-y-2">
      <Handle type="target" position={Position.Left} id={id} />
      <TextInput
        name="targetTableName"
        placeholder="e.g. orders.orders_pending_hourly"
        defaultValue="e.g. orders.orders_pending_hourly"
        label="Target table name"
        onChange={(event) => console.log(event.currentTarget.value)}
      />
      <Handle type="source" position={Position.Right} id={id} />
    </div>
    </div>
  );
});

const ViewNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }
  return (
    <div className={classNames}>
      <Handle type="target" position={Position.Left} id={id} />
      <TextArea
        label="View"
        placeholder="SQL query"
        name="sql_query"
        onChange={(event) => console.log(event.currentTarget.value)}
      />
      <Handle type="source" position={Position.Right} id={id} />
    </div>
  );
});

export {
  DatabaseSourceNode,
  DatabaseSinkNode,
  SourceTableNode,
  TargetTableNode,
  ViewNode,
  SqliteSourceNode,
  SqliteDestinationNode,
  MycelialNetworkNode,
  KafkaSourceNode,
  SnowflakeSourceNode,
  SnowflakeDestinationNode,
};
