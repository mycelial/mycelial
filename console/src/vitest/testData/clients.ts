const sampleClientData = [
  {
    type: 'sqlite_connector',
    display_name: 'Sqlite Source',
    path: 'test.sqlite',
    id: 'Sqlite Source sqlite_connector',
    source: true,
  },
  {
    type: 'sqlite_physical_replication',
    display_name: 'Sqlite Physical Replication Movie',
    journal_path: '/tmp/something.sqlite.mycelial_src',
    id: 'Sqlite Physical Replication Movie sqlite_physical_replication',
    source: true,
    destination: true,
  },
  {
    type: 'hello_world',
    display_name: 'Hello World Src',
    id: 'Hello World Src hello_world',
    source: true,
  },
  { type: 'hello_world', display_name: 'Hello World Dest', destination: true },
];

const sampleClientResponse = {
  data: {
    clients: [
      {
        id: 'ui',
        display_name: 'UI',
        sources: [],
        destinations: [],
      },
      {
        id: 'dev',
        display_name: 'Dev',
        sources: [
          {
            type: 'sqlite_connector',
            display_name: 'Sqlite Source',
            path: 'test.sqlite',
          },
          {
            type: 'sqlite_physical_replication',
            display_name: 'Sqlite Physical Replication Movie',
            journal_path: '/tmp/something.sqlite.mycelial_src',
          },
          { type: 'hello_world', display_name: 'Hello World Src' },
        ],
        destinations: [
          {
            type: 'sqlite_physical_replication',
            display_name: 'Sqlite Physical Replication Movie',
            journal_path: '/tmp/something.sqlite.mycelial',
            database_path: '/tmp/hydrated_db.sqlite',
          },
          { type: 'hello_world', display_name: 'Hello World Dest' },
        ],
      },
    ],
  },
};

export {
    sampleClientResponse,
  sampleClientData,
};
