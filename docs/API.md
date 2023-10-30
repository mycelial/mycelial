# Mycelial API

## Pipeline Specification (workflows)

<details>
  <summary><code>POST</code> <code><b>/api/pipe</b></code> <code>Creates a pipe config</code></summary>

### Headers

> | name          | type     | data type | description          |
> | ------------- | -------- | --------- | -------------------- |
> | Authorization | required | string    | Base64 encoded token |

### Parameters

> | name | type     | data type             | descr openiption |
> | ---- | -------- | --------------------- | ---------------- |
> | None | required | object/payload (JSON) | N/A              |

#### Payloads

<details>
  <summary>Mycelite Source</summary>

```json
{
  "configs": [
    {
      "workspace_id": 1, // this field is currently optional and will default to 1, but may be required in the future.
      "pipe": [
        {
          "name": "sqlite_physical_replication_source",
          "label": "sqlite_physical_replication_source node",
          "client": "{client name}",
          "type": "sqlite_physical_replication",
          "display_name": "{display name}",
          "journal_path": "{path and filename of source journal"
        },
        {
          "name": "mycelial_server_destination",
          "label": "mycelial_server node",
          "type": "mycelial_server",
          "display_name": "Mycelial Server",
          "endpoint": "http://{host or ip}:8080/ingestion",
          "token": "{security token}",
          "topic": "{unique topic id}"
        }
      ]
    }
  ]
}
```

</details>

<details>
  <summary>Mycelite Destination</summary>

```json
{
  "configs": [
    {
      "workspace_id": 1, // this field is currently optional and will default to 1, but may be required in the future.
      "pipe": [
        {
          "name": "mycelial_server_source",
          "label": "mycelial_server node",
          "type": "mycelial_server",
          "display_name": "Mycelial Server",
          "endpoint": "http://{host or ip}:8080/ingestion",
          "token": "token",
          "topic": "{topic id}"
        },
        {
          "name": "sqlite_physical_replication_destination",
          "label": "sqlite_physical_replication_destination node",
          "client": "dev",
          "type": "sqlite_physical_replication",
          "display_name": "{display name}",
          "journal_path": "{path and filename of destination journal}",
          "database_path": "{path and filename of destination database"
        }
      ]
    }
  ]
}
```

</details>

### Responses

> | http code | content-type               | response                             |
> | --------- | -------------------------- | ------------------------------------ |
> | `200`     | `application/json`         | `Configuration created successfully` |
> | `400`     | `text/plain;charset=UTF-8` |                                      |

### Example cURL

> ```bash
>  curl -X POST 'http://{server}:8080/api/pipe' -H 'Authorization: Basic {base 64 token:}' --data @post.json'
> ```

</details>

<details>
  <summary><code>PUT</code> <code><b>/api/pipe/{id}</b></code> <code>Updates a pipe config</code></summary>

### Headers

> | name          | type     | data type | description          |
> | ------------- | -------- | --------- | -------------------- |
> | Authorization | required | string    | Base64 encoded token |

### Parameters

> | name | type     | data type             | description |
> | ---- | -------- | --------------------- | ----------- |
> | None | required | object/payload (JSON) | N/A         |

#### Payloads

<details>
  <summary>Mycelite Source</summary>

```json
{
  "configs": [
    {
      "workspace_id": 1, // this field is currently optional and will default to 1 if not provided, but may be required in the future.
      "pipe": [
        {
          "name": "sqlite_physical_replication_source",
          "label": "sqlite_physical_replication_source node",
          "client": "{client name}",
          "type": "sqlite_physical_replication",
          "display_name": "{display name}",
          "journal_path": "{path and filename of source journal"
        },
        {
          "name": "mycelial_server_destination",
          "label": "mycelial_server node",
          "type": "mycelial_server",
          "display_name": "Mycelial Server",
          "endpoint": "http://{host or ip}:8080/ingestion",
          "token": "{security token}",
          "topic": "{unique topic id}"
        }
      ]
    }
  ]
}
```

</details>

<details>
  <summary>Mycelite Destination</summary>

```json
{
  "configs": [
    {
      "workspace_id": 1, // this field is currently optional and will default to 1 if not provided, but may be required in the future.
      "pipe": [
        {
          "name": "mycelial_server_source",
          "label": "mycelial_server node",
          "type": "mycelial_server",
          "display_name": "Mycelial Server",
          "endpoint": "http://{host or ip}:8080/ingestion",
          "token": "token",
          "topic": "{topic id}"
        },
        {
          "name": "sqlite_physical_replication_destination",
          "label": "sqlite_physical_replication_destination node",
          "client": "dev",
          "type": "sqlite_physical_replication",
          "display_name": "{display name}",
          "journal_path": "{path and filename of destination journal}",
          "database_path": "{path and filename of destination database"
        }
      ]
    }
  ]
}
```

</details>

### Responses

> | http code | content-type               | response                             |
> | --------- | -------------------------- | ------------------------------------ |
> | `200`     | `application/json`         | `Configuration created successfully` |
> | `400`     | `text/plain;charset=UTF-8` |                                      |

### Example cURL

> ```bash
>  curl -X POST 'http://{server}:8080/api/pipe/1' -H 'Authorization: Basic {base 64 token:}' --data @post.json'
> ```

</details>

<details>
  <summary><code>DELETE</code> <code><b>/api/pipe/{id}</b></config></code> <code>Delete a config</code></summary>

### Parameters

> None

### Responses

> | http code | content-type               | response |
> | --------- | -------------------------- | -------- |
> | `200`     | `text/plain;charset=UTF-8` |          |

##### Example cURL

> ```bash
>  curl 'http://localhost:8080/api/pipe/{id}' -X 'DELETE' -H 'Authorization: Basic {base 64 token:}' \
> ```

</details>

<details>
 <summary><code>GET</code> <code><b>/api/pipe/{id}</b></code> <code>fetch a single pipeline specification by id</code></summary>

##### Parameters

> None

##### Responses

> | http code | content-type       | response              |
> | --------- | ------------------ | --------------------- |
> | `200`     | `application/json` | active configurations |

##### Example cURL

> ```bash
>  curl 'http://{server}:8080/api/pipe/{id}' -H 'Authorization: Basic {base 64 token:}'
> ```

</details>

<details>
 <summary><code>GET</code> <code><b>/api/pipe</b></code> <code>(fetch all active pipeline specifications)</code></summary>

##### Parameters

> None

##### Responses

> | http code | content-type       | response              |
> | --------- | ------------------ | --------------------- |
> | `200`     | `application/json` | active configurations |

##### Example cURL

> ```bash
>  curl 'http://{server}:8080/api/pipe' -H 'Authorization: Basic {base 64 token:}'
> ```

</details>

---

## Clients

<details>
  <summary><code>GET</code> <code><b>/api/clients</b></code> <code>List of registered clients</code></summary>

### Headers

> | name          | type     | data type | description          |
> | ------------- | -------- | --------- | -------------------- |
> | Authorization | required | string    | Base64 encoded token |

### Parameters

> None

### Responses

> | http code | content-type       | response |
> | --------- | ------------------ | -------- |
> | `200`     | `application/json` | JSON     |

<details>
  <summary>Response Example</summary>

> ```js
> {
>     "clients": [
>         {
>             "id": "dev_client",
>             "display_name": "Client 1",
>             "sources": [
>                 {
>                     "type": "sqlite_physical_replication",
>                     "display_name": "Mycelite SRC",
>                     "journal_path": "/Users/knowthen/junk/source.db-mycelial"
>                 }
>             ],
>             "destinations": [
>                 {
>                     "type": "sqlite_physical_replication",
>                     "display_name": "Mycelite DEST",
>                     "journal_path": "/Users/knowthen/junk/dest/destination.db-mycelial",
>                     "database_path": "/Users/knowthen/junk/dest/destination.db"
>                 },
>             ]
>         },
>         {
>             "id": "ui",
>             "display_name": "UI",
>             "sources": [],
>             "destinations": []
>         }
>     ]
> }
> ```

</details>

### Example cURL

> ```bash
>  curl 'http://{server}:8080/api/clients' -H 'Authorization: Basic {base 64 token:}'
> ```

</details>

## Workspaces

<details>
  <summary><code>GET</code> <code><b>/api/workspaces</b></code> <code>Fetches all workspaces (not hydrated with pipe configs)</code></summary>

### Headers

> | name          | type     | data type | description          |
> | ------------- | -------- | --------- | -------------------- |
> | Authorization | required | string    | Base64 encoded token |

### Parameters

> None

### Responses

> | http code | content-type               | response                             |
> | --------- | -------------------------- | ------------------------------------ |
> | `200`     | `application/json`         | `Configuration created successfully` |
> | `400`     | `text/plain;charset=UTF-8` |                                      |

```json
[
  {
    "id": 1,
    "created_at": "2023-10-30T19:46:53.429132Z",
    "pipe_configs": [],
    "name": "Default"
  }
]
```

### Example cURL

> ```bash
>  curl 'http://{server}:8080/api/workspaces' -H 'Authorization: Basic {base 64 token:}'
> ```

</details>

<details>
  <summary><code>GET</code> <code><b>/api/workspaces/{id}</b></code> <code>Fetches a single workspace by id that is hydrated with pipe configs</code></summary>

### Headers

> | name          | type     | data type | description          |
> | ------------- | -------- | --------- | -------------------- |
> | Authorization | required | string    | Base64 encoded token |

### Parameters

> None

### Responses

> | http code | content-type               | response                             |
> | --------- | -------------------------- | ------------------------------------ |
> | `200`     | `application/json`         | `Configuration created successfully` |
> | `400`     | `text/plain;charset=UTF-8` |                                      |

```json
{
  "id": 1,
  "created_at": "2023-10-30T19:55:13.646818Z",
  "pipe_configs": [
    {
      "id": 18,
      "pipe": [
        {
          "client": "dev",
          "display_name": "Excel Source",
          "journal_path": "/tmp/test.xlsx",
          "label": "excel_connector_source node",
          "name": "excel_connector_source",
          "path": "/tmp/test.xlsx",
          "sheets": "Sheet1",
          "type": "excel_connector"
        },
        {
          "client": "dev",
          "display_name": "Hello World Dest",
          "label": "hello_world_destination node",
          "name": "hello_world_destination",
          "type": "hello_world"
        }
      ],
      "workspace_id": 1
    }
  ],
  "name": "Default"
}
```

### Example cURL

> ```bash
>  curl 'http://{server}:8080/api/workspaces/1' -H 'Authorization: Basic {base 64 token:}'
> ```

</details>

<details>
  <summary><code>POST</code> <code><b>/api/workspaces</b></code> <code>Creates a workspace</code></summary>

### Headers

> | name          | type     | data type | description          |
> | ------------- | -------- | --------- | -------------------- |
> | Authorization | required | string    | Base64 encoded token |

### Parameters

> | name | type     | data type             | description |
> | ---- | -------- | --------------------- | ----------- |
> | None | required | object/payload (JSON) | N/A         |

### Responses

> | http code | content-type               | response                             |
> | --------- | -------------------------- | ------------------------------------ |
> | `200`     | `application/json`         | `Configuration created successfully` |
> | `400`     | `text/plain;charset=UTF-8` |                                      |

### Example cURL

> ```bash
>  curl -X POST 'http://{server}:8080/api/workspaces/1' -H 'Authorization: Basic {base 64 token:}' --data-raw $'{"name":"new"}'
> ```

```json
{"id":1,"created_at":"1970-01-01T00:00:00Z","pipe_configs":[],"name":"new"}
```

</details>

<details>
  <summary><code>PUT</code> <code><b>/api/workspaces/{id}</b></code> <code>Update a workspace</code></summary>

### Headers

> | name          | type     | data type | description          |
> | ------------- | -------- | --------- | -------------------- |
> | Authorization | required | string    | Base64 encoded token |

### Parameters

> | name | type     | data type             | description |
> | ---- | -------- | --------------------- | ----------- |
> | None | required | object/payload (JSON) | N/A         |

### Responses

> | http code | content-type               | response                             |
> | --------- | -------------------------- | ------------------------------------ |
> | `200`     | `application/json`         | `Configuration created successfully` |
> | `400`     | `text/plain;charset=UTF-8` |                                      |

### Example cURL

> ```bash
>  curl -X PUT 'http://{server}:8080/api/workspaces/1' -H 'Authorization: Basic {base 64 token:}' --data-raw $'{"name":"rename"}'
> ```

```json
{"id":1,"created_at":"1970-01-01T00:00:00Z","pipe_configs":[],"name":"rename"}
```

</details>

<details>
  <summary><code>DELETE</code> <code><b>/api/workspaces/{id}</b></code> <code>Delete a workspace</code></summary>

### Headers

> | name          | type     | data type | description          |
> | ------------- | -------- | --------- | -------------------- |
> | Authorization | required | string    | Base64 encoded token |

### Parameters

> None

### Responses

> | http code | content-type               | response                             |
> | --------- | -------------------------- | ------------------------------------ |
> | `200`     | `application/json`         | `Configuration created successfully` |
> | `400`     | `text/plain;charset=UTF-8` |                                      |

### Example cURL
> ```bash
>  curl -X DELETE 'http://{server}:8080/api/workspaces/1' -H 'Authorization: Basic {base 64 token:}''
> ```
