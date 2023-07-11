import { memo, FC, useEffect, useCallback } from 'react';
import { Handle, Position, NodeProps, NodeResizer, useNodeId, useReactFlow } from 'reactflow';

import { Select, MultiSelect, Textarea, TextInput } from '@/components/inputs';

import styles from '@/components/Flow/Flow.module.css';


const SqliteSourceNode: FC<NodeProps> = memo(({ id, data }) => {
  const instance = useReactFlow();

  let defaultValues = {
    databasePath: "/tmp/test.sqlite",
    sqlQuery: "select * from test;"
  };

  const handleChange = useCallback((name: string, value: string) => {
    instance.setNodes((nodes) => nodes.map((node) => {
      if (node.id === id) {
        node.data = {
          ...node.data,
          [name]: value
        }
      }

      return node;
    }));
  }, []);

  useEffect(() => {
    handleChange("path", defaultValues.databasePath);
    handleChange("query", defaultValues.sqlQuery);
  }, [id]);

  return (
    <div className={styles.customNode}>
      <TextInput
        label="SQLite Database Path"
        placeholder={defaultValues.databasePath}
        defaultValue={defaultValues.databasePath}
        onChange={(event) => handleChange("path", event.currentTarget.value)}
      />
      <TextInput
        label="SQL query"
        placeholder={defaultValues.sqlQuery}
        defaultValue={defaultValues.sqlQuery}
        onChange={(event) => handleChange("query", event.currentTarget.value)}
      />
      <Handle type="source" position={Position.Right} id={id} />
    </div>
  )
});

const MycelialNetworkNode: FC<NodeProps> = memo(({ id, data }) => {
  const instance = useReactFlow();

  let defaultValues = {
    endpoint: "http://localhost:8000/ingestion",
    token: "..."
  };

  const handleChange = useCallback((name: string, value: string) => {
    instance.setNodes((nodes) => nodes.map((node) => {
      console.log(id, node.id);

      if (node.id === id) {
        node.data = {
          ...node.data,
          [name]: value
        }
      }

      return node;
    }));
  }, []);

  useEffect(() => {
    handleChange("endpoint", defaultValues.endpoint);
    handleChange("token", defaultValues.token);
  }, [id]);

  return (
    <div className={styles.customNode}>
      <Handle type="target" position={Position.Left} id={id} />
      <TextInput
        label="Mycelial Network Endpoint"
        placeholder={defaultValues.endpoint}
        defaultValue={defaultValues.endpoint}
        onChange={(event) => handleChange('endpoint', event.currentTarget.value)}
      />
      <TextInput
        label="Token"
        placeholder={defaultValues.token}
        defaultValue={defaultValues.token}
        onChange={(event) => handleChange('token', event.currentTarget.value)}
      />
    </div>
  )
});

const DatabaseSourceNode: FC<NodeProps> = memo(({ id, data }) => {
  const instance = useReactFlow();
  
  // console.log('DatabaseSourceNode.data', data, instance, instance.getEdges());

  useEffect(() => {
    instance.setNodes((nodes) => nodes.map((node) => {
      if (node.id === id) {
        node.data = {
          ...node.data,
          customId: id
        }
      }

      return node;
    }));
  }, [id]);

  return (
    <div className={styles.customNode}>
      <Select
        label="Database Source"
        placeholder="Pick one"
        searchable
        nothingFound="No options"
        data={['Sqlite', 'PostgreSQL', 'Snowflake']}
        withinPortal
      />
      <MultiSelect
        label="Node labels"
        placeholder="Pick multiple"
        searchable
        nothingFound="No options"
        data={['device:drone', 'size:xs', 'sensor:temperature']}
        withinPortal
      />
      <Handle type="source" position={Position.Right} id={id} />
    </div>
  );
})

const DatabaseSinkNode: FC<NodeProps> = memo(({ id, data }) => {
  return (
    <div className={styles.customNode}>
      <Handle type="target" position={Position.Left} id={id} />
      <Select
        label="Database Sink"
        placeholder="Pick one"
        searchable
        nothingFound="No options"
        data={['Snowflake']}
        withinPortal
      />
    </div>
  );
})

const SourceTableNode: FC<NodeProps> = memo((props) => {
  const { id, data } = props;

  console.log(props);

  return (
    <div className={styles.customNode}>
      <Handle type="target" position={Position.Left} id={id} />
      <Select
        label="Table"
        placeholder="Pick multiple"
        searchable
        nothingFound="No options"
        data={['users', 'orders', 'orders_pending', 'orders_complete']}
        withinPortal
      />
      <Handle type="source" position={Position.Right} id={id} />
    </div>
  );
})

const TargetTableNode: FC<NodeProps> = memo(({ id, data }) => {
  return (
    <div className={styles.customNode}>
      <Handle type="target" position={Position.Left} id={id} />
      <TextInput
        placeholder="e.g. orders.orders_pending_hourly"
        label="Target table name"
      />
      <Handle type="source" position={Position.Right} id={id} />
    </div>
  );
})

const ViewNode: FC<NodeProps> = memo(({ id, data }) => {
  return (
    <div className={styles.customNode}>
      <Handle type="target" position={Position.Left} id={id} />
      <Textarea
        placeholder="SQL query"
        label="View" />
      <Handle type="source" position={Position.Right} id={id} />
    </div>
  );
})

export {
  DatabaseSourceNode,
  DatabaseSinkNode,
  SourceTableNode,
  TargetTableNode,
  ViewNode,
  SqliteSourceNode,
  MycelialNetworkNode,
};