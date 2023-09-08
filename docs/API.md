# Mycelial API

## CONFIGURATION 

<details>
  <summary><code>POST</code> <code><b>/api/pipe/configs</b></code> <code>Creates or updates config</code></summary>

### Headers
> | name      |  type     | data type               | description                                                          |
> |-----------|-----------|-------------------------|-----------------------------------------------------------------------|
> | Authorization|  required | string               | Base64 encoded token  |

### Parameters

> | name      |  type     | data type               | description                                                           |
> |-----------|-----------|-------------------------|-----------------------------------------------------------------------|
> | None      |  required | object/payload (JSON)   | N/A  |

#### Payloads

<details>
  <summary>SQLite Source</summary>

```js
{
  "configs": [
    {
      "id": 0,
      "pipe": [
        {
          "name": "sqlite_source",
          "client": "{client_name}",
          "path": "{path_to_sqlite}",
          "tables": "{tables}"
        },
        {
          "name": "mycelial_net_destination",
          "endpoint": "http://{server}:8080/ingestion",
          "topic": "{unique_topic_id}"
        }
      ]
    }
  ]
}
```

</details>

<details>
  <summary>SQLite Destination</summary>

```js
{
  "configs": [
    {
      "id": 0,
      "pipe": [
        {
          "name": "mycelial_net_source",
          "endpoint": "http://{server}:8080/ingestion",
          "topic": "{matching_topic_id}"
        },
        {
          "name": "sqlite_destination",
          "path": "{path_to_sqlite}",
          "client": "{client_name}"
        }
      ]
    }
  ]
}
```

</details>

<details>
  <summary>Mycelite Source and Destination</summary>

```js
{
  "configs": [
    {
      "id": 0,
      "pipe": [
        {
          "name": "mycelite_source",
          "client": "{client name}",
          "journal_path": "{path to mycelite journal}"
        },
        {
          "name": "mycelite_destination",
          "client": "{client name}",
          "journal_path": "{path to mycelite journal}",
          "database_path": "{path to sqlite database}"
        }
      ]
    }
  ]
}
```

</details>

<details>
  <summary>Snowflake Source and Destination</summary>

```js
{
  "configs": [
    {
      "id": 0,
      "pipe": [
        {
          "name": "snowflake_source",
          "username": "{snowflake account name}",
          "password": "{snowflake account password",
          "role": "{snowflake role}",
          "account_identifier": "{snowflake account identifier}",
          "warehouse": "{warehouse name}",
          "database": "{database name}",
          "schema": "{database schema}",
          "query": "{sql query}",
          "client": "{client name}"
        },
        {
          "name": "snowflake_destination",
          "username": "{snowflake account name}",
          "password": "{snowflake account password}",
          "role": "{snowflake role}",
          "account_identifier": "{snowflake account identifier}",
          "warehouse": "{warehouse name}",
          "database": "{database name}",
          "schema": "{database schema}",
          "table": "{destination table name}"
        }
      ]
    }
  ]
}
```

</details>

### Responses

> | http code     | content-type                      | response                                                            |
> |---------------|-----------------------------------|---------------------------------------------------------------------|
> | `200`         | `application/json`                | `Configuration created successfully`                                |
> | `400`         | `text/plain;charset=UTF-8`                |                             |

### Example cURL

> ```bash
>  curl -X POST 'http://{server}:8080/api/pipe/configs' -H 'Authorization: Basic {base 64 token:}' --data @post.json'
> ```

</details>

<details>
  <summary><code>DELETE</code> <code><b>/api/pipe/configs/{id}</b></config></code> <code>Delete a config</code></summary>

### Parameters

> None

### Responses

> | http code     | content-type                      | response                                                            |
> |---------------|-----------------------------------|---------------------------------------------------------------------|
> | `200`         | `text/plain;charset=UTF-8`        |                                 |

##### Example cURL

> ```bash
>  curl 'http://localhost:8080/api/pipe/configs/{id}' -X 'DELETE' -H 'Authorization: Basic {base 64 token:}' \
> ```

</details>

<details>
 <summary><code>GET</code> <code><b>/api/pipe/configs</b></code> <code>(fetch all active configurations)</code></summary>

##### Parameters

> None

##### Responses

> | http code     | content-type                      | response                                                            |
> |---------------|-----------------------------------|---------------------------------------------------------------------|
> | `200`         | `application/json`        | active configurations

##### Example cURL

> ```bash
>  curl 'http://{server}:8080/api/pipe/configs' -H 'Authorization: Basic {base 64 token:}'
> ```

</details>

------------------------------------------------------------------------------------------

## Clients

<details>
  <summary><code>GET</code> <code><b>/api/clients</b></code> <code>List of registered clients</code></summary>

### Headers
> | name      |  type     | data type               | description                                                          |
> |-----------|-----------|-------------------------|-----------------------------------------------------------------------|
> | Authorization|  required | string               | Base64 encoded token  |

### Parameters

> None

### Responses

> | http code     | content-type                      | response                                                            |
> |---------------|-----------------------------------|---------------------------------------------------------------------|
> | `200`         | `application/json`                | JSON                                |


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
>                     "type": "sqlite",
>                     "display_name": "Source 1",
>                     "path": "/tmp/test.sqlite"
>                 },
>                 {
>                     "type": "mycelite",
>                     "display_name": "Mycelite SRC",
>                     "journal_path": "/Users/knowthen/junk/source.db-mycelial"
>                 }
>             ],
>             "destinations": [
>                 {
>                     "type": "mycelite",
>                     "display_name": "Mycelite DEST",
>                     "journal_path": "/Users/knowthen/junk/dest/destination.db-mycelial",
>                     "database_path": "/Users/knowthen/junk/dest/destination.db"
>                 },
>                 {
>                     "type": "sqlite",
>                     "display_name": "Dest 1",
>                     "path": "/tmp/test_dest.sqlite"
>                 }
>             ]
>         },
>         {
>             "id": "ui",
>             "display_name": "UI",
>             "sources": [],
>             "destinations": []
>         }
>     ]
}
> ```

</details>

### Example cURL

> ```bash
>  curl 'http://{server}:8080/api/clients' -H 'Authorization: Basic {base 64 token:}'
> ```

</details>


