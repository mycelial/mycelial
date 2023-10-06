# Mycelite

[Mycelite](https://github.com/mycelial/mycelite) is an open-source SQLite
extension that allows you to replicate a SQLite database from a source to a
destination.  Used in conjunction with Mycelial, Mycelite allows you to fully 
replicate a SQLite database from a read/write source to a read-only destination.

## Mycelite extension setup

### Downloading the extension

Download the appropriate build to the computer with the source database

<details>
  <summary>Mac Arm</summary>

```toml
curl -L https://github.com/mycelial/mycelite/releases/latest/download/aarch64-apple-darwin.tgz --output aarch64-apple-darwin.tgz
tar -xvzf aarch64-apple-darwin.tgz
```
</details>
<details>
  <summary>Mac x86</summary>

```toml
curl -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-apple-darwin.tgz --output x86_64-apple-darwin.tgz
tar -xvzf x86_64-apple-darwin.tgz
```
</details>
<details>
  <summary>Linux Arm 32</summary>

```toml
curl -L https://github.com/mycelial/mycelite/releases/latest/download/arm-unknown-linux-gnueabihf.tgz --output arm-unknown-linux-gnueabihf.tgz 
tar -xvzf arm-unknown-linux-gnueabihf.tgz 
```
</details>
<details>
  <summary>Linux Arm 64</summary>

```toml
curl -L https://github.com/mycelial/mycelite/releases/latest/download/aarch64-unknown-linux-gnu.tgz --output arm-unknown-linux-gnueabihf.tgz 
tar -xvzf arm-unknown-linux-gnueabihf.tgz 
```
</details>
<details>
  <summary>Linux x86 gnu</summary>

```toml
curl -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-unknown-linux-gnu.tgz --output x86_64-unknown-linux-gnu.tgz 
tar -xvzf x86_64-unknown-linux-gnu.tgz 
```
</details>
<details>
  <summary>Linux x86 musl</summary>

```toml
curl -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-unknown-linux-musl.tgz --output x86_64-unknown-linux-musl.tgz 
tar -xvzf x86_64-unknown-linux-musl.tgz  
```
</details>
<details>
  <summary>Windows x86 gnu</summary>

```toml
curl.exe -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-pc-windows-gnu.zip --output x86_64-pc-windows-gnu.zip 
tar.exe -xvzf x86_64-pc-windows-gnu.zip
```
</details>
<details>
  <summary>Windows x86 msvc</summary>

```toml
curl.exe -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-pc-windows-msvc.zip --output x86_64-pc-windows-msvc.zip 
tar.exe -xvzf x86_64-pc-windows-msvc.zip
```
</details>

### Load the extension and open the database

After you've downloaded and unzipped the extension, you'll need to load the
extension and open your SQLite database. When the extension is loaded and the
SQLite database is opened, it will create a Mycelite journal file, which is a
sibling file to the SQLite database file. Make a **note** of the journal
`filename` as it will need to be referenced when setting up your pipeline
specification in Mycelial.


<details>
  <summary>Command Line</summary>

```sh
# You must load the extension every time you open the SQLite database
sqlite3
.load ./libmycelite mycelite_writer
.open writer.db
```
</details>

<details>
  <summary>Node.js</summary>

#### Install better-sqlite3

```bash
npm install better-sqlite3
```

#### Setup the writer

##### Import better-sqlite

```js
import Database from 'better-sqlite3';
```

##### Load the extension

```js
let db = new Database(':memory:');

db.loadExtension('./libmycelite', 'mycelite_writer');
```

##### Open the database

```js
db = new Database('writer.db');
```

</details>

<details>
  <summary>Python</summary>

#### Import sqlite3

```python
import sqlite3
```

#### Load the extension

```python
connection = sqlite3.connect(":memory:")
connection.enable_load_extension(True)
connection.execute("select load_extension('./libmycelite', 'mycelite_writer')")
```

#### Open the database

```python
db = sqlite3.connect("writer.db")
```

</details>

### Configure the Mycelial client

Follow the [Setup](Setup.md) guide instructions for setting up the Mycelial
client. Reference the [Client Config](client_config.md) document when
configuring the client.

### Create a SQLite to SQLite data pipeline specification in the web interface

After configuring the Mycelial client, open the [server
console](http://localhost:8080) and perform the following steps.

1. Drag and drop your source node onto the canvas.
2. Drag and drop the Mycelial Server node onto the canvas.
3. Drag and drop your destination node onto the canvas.
4. Connect the source and destination nodes, via the Mycelial Server node
5. Publish your configuration

After performing the above steps, your source SQLite database is replicated in 
near real-time to the destination SQLite database.

### Create a SQLite to SQLite data pipeline specification via the API

If you prefer to set up your data pipelines with an [API](API.md) call, do the
following:

Create the pipeline by making a `POST` specification API calls to
`/api/pipe/configs` with the following payloads:

#### Source to Mycelial Server 

```json
{
  "configs": [
    {
      "id": 0,
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

### Mycelial Server to destination

```json
{
  "configs": [
    {
      "id": 0,
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