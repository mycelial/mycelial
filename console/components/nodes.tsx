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
import { XMarkIcon } from "@heroicons/react/20/solid";

import { Select, MultiSelect, TextArea, TextInput } from "@/components/inputs";

import styles from "@/components/Flow/Flow.module.css";
import { ClientContext } from "./context/clientContext";
import { ClientContextType } from "./@types/client";

const HelloWorldDestinationNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
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
      }),
    );
  }, []);

  useEffect(() => {
    handleChange("client", initialValues.client);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

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
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">HelloWorld Destination</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
        </button>
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
});
const HelloWorldSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      client: data.client ? data.client : "-",
      interval_milis: data.interval_milis ? data.interval_milis : 5000,
      message: data.message ? data.message : "Hello!",
    };
  }, []);

  const handleChange = useCallback((name: string, value: string | number) => {
    instance.setNodes((nodes) =>
      nodes.map((node) => {
        if (node.id === id) {
          node.data = {
            ...node.data,
            [name]: value,
          };
        }

        return node;
      }),
    );
  }, []);

  useEffect(() => {
    handleChange("client", initialValues.client);
    handleChange("interval_milis", initialValues.interval_milis);
    handleChange("message", initialValues.message);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

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
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">HelloWorld Source</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
        </button>
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
        <TextInput
          name="message"
          label="Message"
          placeholder={initialValues.message}
          defaultValue={initialValues.message}
          onChange={(event) => {
            handleChange("message", event.currentTarget.value)
          }}
        />
        <TextInput
          name="interval_milis"
          label="Interval in miliseconds"
          placeholder={initialValues.interval_milis}
          defaultValue={initialValues.interval_milis}
          onChange={(event) => {
            let value = parseInt(event.currentTarget.value);
            handleChange("interval_milis", value)
          }}
        />
        <Handle type="source" position={Position.Right} id={id} />
      </div>
    </div>
  );
});

const SqliteConnectorSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      database_path: data.path ? data.path : "/tmp/test.sqlite",
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
      }),
    );
  }, []);

  useEffect(() => {
    handleChange("path", initialValues.database_path);
    handleChange("tables", initialValues.tables);
    handleChange("client", initialValues.client);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

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
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">SQLite Source</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
        </button>
        <TextInput
          name="sqlitedatabase_path"
          label="SqliteConnector Database Path"
          placeholder={initialValues.database_path}
          defaultValue={initialValues.database_path}
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

const SqliteConnectorDestinationNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      database_path: data.path ? data.path : "/tmp/test_dst.sqlite",
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
      }),
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
    handleChange("path", initialValues.database_path);
    handleChange("client", initialValues.client);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">SQLite Destination</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
        </button>
        <TextInput
          name="sqlitedatabase_path"
          label="SqliteConnector Database Path"
          placeholder={initialValues.database_path}
          defaultValue={initialValues.database_path}
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
});

const ExcelSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      path: data.path ? data.path : "/tmp/test.xlsx",
      client: data.client ? data.client : "-",
      sheets: data.sheets ? data.sheets : "*",
      strict: data.strict ? data.strict : "true",
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
      }),
    );
  }, []);

  useEffect(() => {
    handleChange("journal_path", initialValues.path);
    handleChange("sheets", initialValues.sheets);
    handleChange("client", initialValues.client);
    handleChange("strict", initialValues.strict);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

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
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">Excel Source</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
        </button>
        <TextInput
          name="path"
          label="Excel Path"
          placeholder={initialValues.path}
          defaultValue={initialValues.path}
          onChange={(event) =>
            handleChange("path", event.currentTarget.value)
          }
        />
        <TextInput
          name="sheets"
          label="Sheets"
          placeholder={initialValues.sheets}
          defaultValue={initialValues.sheets}
          onChange={(event) =>
            handleChange("sheets", event.currentTarget.value)
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
        <Select
          name="strict"
          label="Strict"
          placeholder="Pick one"
          defaultValue={initialValues.strict}
          options={["true", "false"]}
          onChange={(value) => {
            handleChange("strict", value || "");
          }}
        />
        <Handle type="source" position={Position.Right} id={id} />
      </div>
    </div>
  );
});

const SqlitePhysicalReplicationSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      journal_path: data.journal_path ? data.journal_path : "/tmp/journal",
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
      }),
    );
  }, []);

  useEffect(() => {
    handleChange("journal_path", initialValues.journal_path);
    handleChange("client", initialValues.client);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

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
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">SqlitePhysicalReplication Source</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
        </button>
        <TextInput
          name="journal_path"
          label="SqlitePhysicalReplication Journal Path"
          placeholder={initialValues.journal_path}
          defaultValue={initialValues.journal_path}
          onChange={(event) =>
            handleChange("journal_path", event.currentTarget.value)
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

const SqlitePhysicalReplicationDestinationNode: FC<NodeProps> = memo(
  ({ id, data, selected }) => {
    const instance = useReactFlow();
    const { clients } = useContext(ClientContext) as ClientContextType;

    let initialValues = useMemo(() => {
      return {
        journal_path: data.journal_path
          ? data.journal_path
          : "/tmp/mycelite_journal_dst",
        database_path: data.database_path ? data.database_path : "",
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
        }),
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
      handleChange("database_path", initialValues.database_path);
      handleChange("journal_path", initialValues.journal_path);
      handleChange("client", initialValues.client);
    }, []);

    let classNames = `${styles.customNode} `;
    if (selected) {
      classNames = classNames + `${styles.selected}`;
    }

    return (
      <div className={classNames}>
        <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
          <h2 className="text-slate-400 font-normal">
            SqlitePhysicalReplication Journal Destination
          </h2>
          <button
            onClick={() => {
              if (confirm("Are you sure you want to delete this node?")) {
                removeNode(id);
              }
            }}
            type="button"
            className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
            title="delete"
          >
            <XMarkIcon className="h-5 w-5" aria-hidden="true" />
          </button>
          <TextInput
            name="mycelitejournal_path"
            label="SqlitePhysicalReplication Journal Path"
            placeholder={initialValues.journal_path}
            defaultValue={initialValues.journal_path}
            onChange={(event) =>
              handleChange("journal_path", event.currentTarget.value)
            }
          />
          <TextInput
            name="mycelitedatabase_path"
            label="SqlitePhysicalReplication Database Path"
            placeholder={initialValues.database_path}
            defaultValue={initialValues.database_path}
            onChange={(event) =>
              handleChange("database_path", event.currentTarget.value)
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
          <Handle type="target" position={Position.Left} id={id} />
        </div>
      </div>
    );
  },
);

const MycelialServerNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();

  let initialValues = useMemo(() => {
    return {
      endpoint: data.endpoint
        ? data.endpoint
        : "http://localhost:7777/ingestion",
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
      }),
    );
  }, []);

  useEffect(() => {
    handleChange("endpoint", initialValues.endpoint);
    handleChange("token", initialValues.token);
    handleChange("topic", initialValues.topic);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }
  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <Handle type="target" position={Position.Left} id={id} />
        <Handle type="source" position={Position.Right} id={id} />
        <h2 className="text-slate-400 font-normal">Mycelial Server</h2>

        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
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

    let initialValues = useMemo(() => {
      return {
        username: data.username ? data.username : "username",
        password: data.password ? data.password : "password",
        role: data.role ? data.role : "role",
        account_identifier: data.account_identifier
          ? data.account_identifier
          : "account_identifier",
        warehouse: data.warehouse ? data.warehouse : "warehouse",
        database: data.database ? data.database : "database",
        schema: data.schema ? data.schema : "schema",
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
        }),
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
    }, []);

    let classNames = `${styles.customNode} `;
    if (selected) {
      classNames = classNames + `${styles.selected}`;
    }

    const removeNode = useCallback((id: string) => {
      let node = instance.getNode(id);
      if (node === undefined) {
        return;
      }
      let edges = getConnectedEdges([node], []);
      instance.deleteElements({ edges, nodes: [node] });
    }, []);

    return (
      <div className={classNames}>
        <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
          <h2 className="text-slate-400 font-normal">Snowflake Destination</h2>
          <button
            onClick={() => {
              if (confirm("Are you sure you want to delete this node?")) {
                removeNode(id);
              }
            }}
            type="button"
            className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
            title="delete"
          >
            <XMarkIcon className="h-5 w-5" aria-hidden="true" />
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
          <Handle type="target" position={Position.Left} id={id} />
        </div>
      </div>
    );
  },
);

const SnowflakeSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      username: data.username ? data.username : "username",
      password: data.password ? data.password : "password",
      role: data.role ? data.role : "role",
      account_identifier: data.account_identifier
        ? data.account_identifier
        : "account_identifier",
      warehouse: data.warehouse ? data.warehouse : "warehouse",
      database: data.database ? data.database : "database",
      schema: data.schema ? data.schema : "schema",
      query: data.query ? data.query : "select 1",
      delay: data.delay ? data.delay : 5,
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
      }),
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
    handleChange("client", initialValues.client);
    handleChange("delay", initialValues.delay);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }
  const removeNode = useCallback((id: string) => {
    let node = instance.getNode(id);
    if (node === undefined) {
      return;
    }
    let edges = getConnectedEdges([node], []);
    instance.deleteElements({ edges, nodes: [node] });
  }, []);

  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">Snowflake Source</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
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
        <TextInput
          name="delay"
          label="delay in seconds"
          placeholder={initialValues.delay}
          defaultValue={initialValues.delay}
          onChange={(event) => handleChange("delay", event.currentTarget.value)}
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

const KafkaDestination: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      brokers: data.brokers ? data.brokers : "localhost:9092",
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
      }),
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
    handleChange("topics", initialValues.topics);
    handleChange("client", initialValues.client);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }
  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">Kafka Destination</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
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
        <Handle type="target" position={Position.Left} id={id} />
      </div>
    </div>
  );
});

const KafkaSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

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
      }),
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

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }
  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">Kafka Source</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
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

const PostgresConnectorSourceNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      url: data.url ? data.url : "postgres://user:password@localhost:5432/test",
      schema: data.schema ? data.schema : "public",
      tables: data.tables ? data.tables : "*",
      client: data.client ? data.client : "-",
      poll_interval: data.poll_interval ? data.poll_interval : 5,
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
      }),
    );
  }, []);

  useEffect(() => {
    handleChange("url", initialValues.url);
    handleChange("schema", initialValues.schema);
    handleChange("tables", initialValues.tables);
    handleChange("client", initialValues.client);
    handleChange("poll_interval", initialValues.poll_interval);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

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
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">SQLite Source</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
        </button>
        <TextInput
          name="url"
          label="Url"
          placeholder={initialValues.url}
          defaultValue={initialValues.url}
          onChange={(event) => handleChange("url", event.currentTarget.value)}
        />
        <TextInput
          name="Schema"
          label="Schema"
          placeholder={initialValues.schema}
          defaultValue={initialValues.schema}
          onChange={(event) => handleChange("schema", event.currentTarget.value)}
        />
        <TextInput
          name="tables"
          label="Tables"
          placeholder={initialValues.tables}
          defaultValue={initialValues.tables}
          onChange={(event) =>
            handleChange("tables", event.currentTarget.value)
          }
        />
        <TextInput
          name="poll_interval"
          label="Poll Interval"
          placeholder={initialValues.poll_interval}
          defaultValue={initialValues.poll_interval}
          onChange={(event) => {
            handleChange("poll_interval", event.currentTarget.value);
          }}
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

const PostgresConnectorDestinationNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      url: data.url? data.url: "postgres://root:root@127.0.0.1:5432/test",
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
      }),
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
    handleChange("url", initialValues.url);
    handleChange("client", initialValues.client);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">Postgres Destination</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
        </button>
        <TextInput
          name="url"
          label="Postgres Connector Database Url"
          placeholder={initialValues.url}
          defaultValue={initialValues.url}
          onChange={(event) => handleChange("url", event.currentTarget.value)}
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
});

const MysqlConnectorDestinationNode: FC<NodeProps> = memo(({ id, data, selected }) => {
  const instance = useReactFlow();
  const { clients } = useContext(ClientContext) as ClientContextType;

  let initialValues = useMemo(() => {
    return {
      url: data.url? data.url: "mysql://root:root@127.0.0.1:3306/test",
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
      }),
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
    handleChange("url", initialValues.url);
    handleChange("client", initialValues.client);
  }, []);

  let classNames = `${styles.customNode} `;
  if (selected) {
    classNames = classNames + `${styles.selected}`;
  }

  return (
    <div className={classNames}>
      <div className=" grid grid-cols-1 gap-x-6 gap-y-2">
        <h2 className="text-slate-400 font-normal">Postgres Destination</h2>
        <button
          onClick={() => {
            if (confirm("Are you sure you want to delete this node?")) {
              removeNode(id);
            }
          }}
          type="button"
          className="absolute right-1 top-1 rounded bg-red-200 text-white shadow-sm hover:bg-red-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-800"
          title="delete"
        >
          <XMarkIcon className="h-5 w-5" aria-hidden="true" />
        </button>
        <TextInput
          name="url"
          label="Mysql Connector Database Url"
          placeholder={initialValues.url}
          defaultValue={initialValues.url}
          onChange={(event) => handleChange("url", event.currentTarget.value)}
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
});

export {
  SqliteConnectorSourceNode,
  SqliteConnectorDestinationNode,
  SqlitePhysicalReplicationSourceNode,
  ExcelSourceNode,
  SqlitePhysicalReplicationDestinationNode,
  MycelialServerNode,
  KafkaSourceNode,
  KafkaDestination,
  SnowflakeSourceNode,
  SnowflakeDestinationNode,
  HelloWorldSourceNode,
  HelloWorldDestinationNode,
  PostgresConnectorSourceNode,
  PostgresConnectorDestinationNode,
  MysqlConnectorDestinationNode
};
