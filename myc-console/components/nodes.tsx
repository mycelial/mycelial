import { memo, FC, useEffect, useCallback } from 'react';
import { Handle, Position, NodeProps, NodeResizer, useNodeId, useReactFlow } from 'reactflow';

import { Select, MultiSelect, Textarea, TextInput } from '@/components/inputs';

import styles from '@/components/Flow/Flow.module.css';


const SqliteSourceNode: FC<NodeProps> = memo(({ id, data }) => {
  const instance = useReactFlow();

  let initialValues = {
    databasePath: data.path ? data.path : "/tmp/test.sqlite",
    sqlQuery: data.query ? data.query : "select * from test;"
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
    handleChange("path", initialValues.databasePath);
    handleChange("query", initialValues.sqlQuery);
  }, [id]);

  return (
    <div className={styles.customNode}>
      <TextInput
        label="SQLite Database Path"
        placeholder={initialValues.databasePath}
        defaultValue={initialValues.databasePath}
        onChange={(event) => handleChange("path", event.currentTarget.value)}
        className="nodrag"
      />
      <TextInput
        label="SQL query"
        placeholder={initialValues.sqlQuery}
        defaultValue={initialValues.sqlQuery}
        onChange={(event) => handleChange("query", event.currentTarget.value)}
        className="nodrag"
      />
      <Handle type="source" position={Position.Right} id={id} />
    </div>
  )
});

const MycelialNetworkNode: FC<NodeProps> = memo(({ id, data }) => {
  const instance = useReactFlow();

  let initialValues = {
    endpoint: data.endpoint ? data.endpoint : "http://localhost:8000/ingestion",
    token: data.token ? data.token : "...",
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
    handleChange("endpoint", initialValues.endpoint);
    handleChange("token", initialValues.token);
  }, [id]);

  return (
    <div className={styles.customNode}>
      <Handle type="target" position={Position.Left} id={id} />
      <TextInput
        className="nodrag"
        label="Mycelial Network Endpoint"
        placeholder={initialValues.endpoint}
        defaultValue={initialValues.endpoint}
        onChange={(event) => handleChange('endpoint', event.currentTarget.value)}
      />
      <TextInput
        className="nodrag"
        label="Token"
        placeholder={initialValues.token}
        defaultValue={initialValues.token}
        onChange={(event) => handleChange('token', event.currentTarget.value)}
      />
    </div>
  )
});


const KafkaSourceNode: FC<NodeProps> = memo(({ id, data }) => {
  const instance = useReactFlow();

  let initialValues = {
    brokers: data.brokers ? data.brokers : "localhost:9092",
    group_id: data.group_id ? data.group_id : "group_id",
    topics: data.topics ? data.topics : "topic-0",
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
    handleChange("brokers", initialValues.brokers);
    handleChange("group_id", initialValues.group_id);
    handleChange("topics", initialValues.topics);
  }, [id]);

  return (
    <div className={styles.customNode}>
      <TextInput
        className="nodrag"
        label="Brokers"
        placeholder={initialValues.brokers}
        defaultValue={initialValues.brokers}
        onChange={(event) => handleChange("brokers", event.currentTarget.value)}
      />
      <TextInput
        className="nodrag"
        label="GroupId"
        placeholder={initialValues.group_id}
        defaultValue={initialValues.group_id}
        onChange={(event) => handleChange("group_id", event.currentTarget.value)}
      />
      <TextInput
        className="nodrag"
        label="Topics"
        placeholder={initialValues.topics}
        defaultValue={initialValues.topics}
        onChange={(event) => handleChange("topics", event.currentTarget.value)}
      />
      <Handle type="source" position={Position.Right} id={id} />
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
        className="nodrag"
        label="Database Source"
        placeholder="Pick one"
        searchable
        nothingFound="No options"
        data={['Sqlite', 'PostgreSQL', 'Snowflake']}
        withinPortal
      />
      <MultiSelect
        className="nodrag"
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
        className="nodrag"
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
        className="nodrag"
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
        className="nodrag"
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
        className="nodrag"
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
  KafkaSourceNode, 
};
