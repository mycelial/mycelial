# Client Configuration File (config.toml)

Mycelial clients require a configuration file to function properly.

You can find a [config.example.toml](client/config.example.toml) example file,
which you can reference when creating your own client configuration files.

The configuration file is passed to the client binary file via the
`./myceliald --config=./config.toml` command line option. 

## Configuration Sections

### Node Section

The first section you'll need to configure is the `node` section. Three
key-value pairs define the client. The `display_name` specifies the
human-readable client name, which is displayed in the server UI. The `unique_id`
is a unique identifier that you assign each client. Lastly, the `storage_path`
specifies the name of the SQLite file to use for the client. The SQLite file 
stores the pipeline specifications that are downloaded from the server.

<details>
  <summary>Example Node Section</summary>

```toml
[node]
display_name = "Client ABC"
unique_id = "client_abc"
storage_path = "client.sqlite"
```
</details>

### Server Section

The `server` section defines two things: The server `endpoint` address and the
security `token`. The security token on the client must match the security token
specified on the server.

<details>
  <summary>Example Server Section</summary>

```toml
[server]
endpoint = "http://localhost:7777"
token = "my security token"
```
</details>

### Source Sections

You can have zero or more `sources` sections, which define which data sources
are available to the client. Currently, there is only one production-ready data
source which is `mycelite`. Mycelite allows you to synchronize one SQLite 
database to another. Note: many other data sources are currently in the works.

#### Mycelite data source

To define a **mycelite** data source, you'll first specify a `type` property
with a value of `sqlite_physical_replication`. The next property you'll define
is the `display_name` which is where you'll specify a human-readable name for
this datasource. The display name is used and displayed in the server UI. The
last property you'll define is the `journal_path`, the value you assign to it is
the full path and file name to the Mycelite journal. Note: the Mycelite journal
is automatically created as a sibling file to the SQLite database file when you
use the Mycelite SQLite extension.
<details>
  <summary>Example Mycelite Source Section</summary>

```toml
[[sources]]
type = "sqlite_physical_replication"
display_name = "Objects Detected"
journal_path = "/tmp/objects_source.sqlite.mycelial"
```
</details>

#### Append only SQLite data source

To define a **SQLite** data source, you'll first specify a `type` property with
a value of `sqlite_connector`. The next property you'll define is the
`display_name` which is where you'll specify a human-readable name for this
datasource. The display name is used and displayed in the server UI. The last 
property you'll define is the `path`, the value you assign to it is the full
path and file name to the SQLite database.

<details>
  <summary>Example Append only SQLite Source Section</summary>

```toml
[[sources]]
type = "sqlite_connector"
display_name = "Detections database"
path = "/tmp/test.sqlite"
```

</details>

### Destination Sections

You can have zero or more `destination` sections, which define which data
destinations are available to the client. Currently, there is only one
production-ready data source, which is `mycelite`.

#### Mycelite data destination

To define a **mycelite** data destination, you'll first specify a `type`
property with a value of `sqlite_physical_replication`. The next property you'll
define is the `display_name` which is where you'll specify a human-readable name
for this data destination. The display name is used and displayed in the server
UI. The next property you'll define is the `journal_path` which you'll set to
the full path and filename of the destination journal. The path must be a valid
directory path on the client and the journal name can be a name of your
choosing. The last property you'll define is the `database_path` which you'll
set to the full path qpand filename of the destination database.

<details>
  <summary>Example Mycelite Destination Section</summary>

```toml
[[destinations]]
type = "sqlite_physical_replication"
display_name = "Objects Detected"
journal_path = "/tmp/objects_dest.sqlite.mycelial"
database_path = "/tmp/hydrated_db.sqlite"
```
</details>

#### Append only SQLite data destination

To define an append only **SQLite** data destination, you'll first specify a
`type` property with a value of `sqlite_connector`. The next property you'll 
define is the `display_name` which is where you'll specify a human-readable name
for this data destination. The display name is used and displayed in the server
UI. The last property you'll define is the `path` which you'll set to the full
path and filename of the destination SQLite database.

<details>
  <summary>Example Append Only SQLite Destination Section</summary>

```toml
[[destinations]]
type = "sqlite_connector"
display_name = "Detections destination"
path = "/tmp/destination.sqlite"
```
</details>

#### Append only Postgres data destination

To define an append only **Postgres** data destination, you'll first specify a
`type` property with a vaule of `postgres_connector`. The next property you'll
define is the `display_name` which is where you'll specify a human-readable name
for this data destination. The display name is used and displayed in the server
UI. The last property you'll define is the `url` which you'll set to be a valid
Postgres connection string in the format
`postgres://user:password@127.0.0.1:5432/database_name`

<details>
  <summary>Example Append Only Postgres Destination Section</summary>

```toml
[[destinations]]
type = "postgres_connector"
display_name = "postgres destination"
url = "postgres://user:password@127.0.0.1:5432/test"
```
</details>
