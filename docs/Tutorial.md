# Tutorial

In this tutorial, we'll setup Mycelial to synchronize data from one SQLite 
instance to another.

## Server

### Download and unarchive the server binary for you system.

   <details>
   <summary>Mac arm_64</summary>

   ```sh
   curl -L https://github.com/mycelial/mycelial/releases/latest/download/server-aarch64-apple-darwin.tgz --output server-aarch64-apple-darwin.tgz
   tar -xvzf server-aarch64-apple-darwin.tgz
   ```

   </details>
   
   <details>
   <summary>Mac x86_64</summary>

   ```sh
   curl -L https://github.com/mycelial/mycelial/releases/latest/download/server-x86_64-apple-darwin.tgz --output server-x86_64-apple-darwin.tgz
   tar -xvzf server-x86_64-apple-darwin.tgz
   ```

   </details>

   <details>
   <summary>Linux x86_64</summary>

   ```sh
   curl -L https://github.com/mycelial/mycelial/releases/latest/download/server-x86_64-unknown-linux-gnu.tgz --output server-x86_64-unknown-linux-gnu.tgz
   tar -xvzf server-x86_64-unknown-linux-gnu.tgz
   ```

   </details>

   <details>
   <summary>Linux arm_64</summary>

   ```sh
   curl -L https://github.com/mycelial/mycelial/releases/latest/download/server-aarch64-unknown-linux-gnu.tgz --output server-aarch64-unknown-linux-gnu.tgz
   tar -xvzf server-aarch64-unknown-linux-gnu.tgz
   ```

   </details>

   <details>
   <summary>Linux arm_32</summary>

   ```sh
   curl -L https://github.com/mycelial/mycelial/releases/latest/download/server-arm-unknown-linux-gnueabihf.tgz --output server-arm-unknown-linux-gnueabihf.tgz
   tar -xvzf server-arm-unknown-linux-gnueabihf.tgz
   ```

   </details>

### Start the server

Start the server binary with the following command `./server --token secrettoken`

## Mycelite

### Download and unarchive the mycelite extension for your system

<details>
  <summary>Mac Arm_64</summary>

  ```toml
  curl -L https://github.com/mycelial/mycelite/releases/latest/download/aarch64-apple-darwin.tgz --output aarch64-apple-darwin.tgz
  tar -xvzf aarch64-apple-darwin.tgz
  ```

</details>
<details>
  <summary>Mac x86_64</summary>

```sh
curl -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-apple-darwin.tgz --output x86_64-apple-darwin.tgz
tar -xvzf x86_64-apple-darwin.tgz
```
</details>
<details>
  <summary>Linux arm_32</summary>

```sh
curl -L https://github.com/mycelial/mycelite/releases/latest/download/arm-unknown-linux-gnueabihf.tgz --output arm-unknown-linux-gnueabihf.tgz 
tar -xvzf arm-unknown-linux-gnueabihf.tgz 
```
</details>
<details>
  <summary>Linux arm_64</summary>

```sh
curl -L https://github.com/mycelial/mycelite/releases/latest/download/aarch64-unknown-linux-gnu.tgz --output arm-unknown-linux-gnueabihf.tgz 
tar -xvzf arm-unknown-linux-gnueabihf.tgz 
```
</details>
<details>
  <summary>Linux x86_gnu</summary>

```sh
curl -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-unknown-linux-gnu.tgz --output x86_64-unknown-linux-gnu.tgz 
tar -xvzf x86_64-unknown-linux-gnu.tgz 
```
</details>
<details>
  <summary>Linux x86_musl</summary>

```sh
curl -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-unknown-linux-musl.tgz --output x86_64-unknown-linux-musl.tgz 
tar -xvzf x86_64-unknown-linux-musl.tgz  
```
</details>
<details>
  <summary>Windows x86_gnu</summary>

```sh
curl.exe -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-pc-windows-gnu.zip --output x86_64-pc-windows-gnu.zip 
tar.exe -xvzf x86_64-pc-windows-gnu.zip
```
</details>
<details>
  <summary>Windows x86_msvc</summary>

```sh
curl.exe -L https://github.com/mycelial/mycelite/releases/latest/download/x86_64-pc-windows-msvc.zip --output x86_64-pc-windows-msvc.zip 
tar.exe -xvzf x86_64-pc-windows-msvc.zip
```
</details>

### Using the Mycelite extension

After you've downloaded and unzipped the extension, you'll need to load the
extension and open your SQLite database. When the extension is loaded and the
SQLite database is opened, it will create a Mycelite journal file, which is a
sibling file to the SQLite database file. Make a **note** of the journal
`filename` as it will need to be referenced when setting up your pipeline
specification in Mycelial.


```sh
sqlite3
.load ./libmycelite mycelite_writer
.open data.db
```

Next, create a table in your sqlite database and insert a record.

```sql
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
INSERT INTO users(name) VALUES ('John');
```

After modifying the database, the Mycelite extension will create a journal file
as a sibling to the database file. In this example setup, the journal file name
should be `data.db-mycelial`. This file will be used to setup the Mycelial 
client in a moment.

## Client (mycelaild)

### Download and unarchive the mycelial client (myceliald) for your system

   <details>
   <summary>Mac arm_64</summary>

   ```sh
   curl -L https://github.com/mycelial/mycelial/releases/latest/download/myceliald-aarch64-apple-darwin.tgz --output myceliald-aarch64-apple-darwin.tgz
   tar -xvzf myceliald-aarch64-apple-darwin.tgz
   ```

   </details>

   <details>
   <summary>Mac x86_64</summary>

   ```sh
   curl -L https://github.com/mycelial/mycelial/releases/latest/download/myceliald-x86_64-unknown-linux-gnu.tgz --output myceliald-x86_64-unknown-linux-gnu.tgz
   tar -xvzf myceliald-x86_64-unknown-linux-gnu.tgz
   ```

   </details>

   <details>
   <summary>Linux x86_64</summary>

   ```sh
   curl -L https://github.com/mycelial/mycelial/releases/latest/download/myceliald-x86_64-unknown-linux-gnu.tgz --output myceliald-x86_64-unknown-linux-gnu.tgz
   tar -xvzf myceliald-x86_64-unknown-linux-gnu.tgz
   ```

   </details>

   <details>
   <summary>Linux arm_32</summary>

   ```sh
   curl -L https://github.com/mycelial/mycelial/releases/latest/download/myceliald-arm-unknown-linux-gnueabihf.tgz --output myceliald-arm-unknown-linux-gnueabihf.tgz
   tar -xvzf myceliald-arm-unknown-linux-gnueabihf.tgz
   ```

   </details>

   <details>
   <summary>Linux arm_64</summary>

   ```sh
   curl -L https://github.com/mycelial/mycelial/releases/latest/download/myceliald-aarch64-unknown-linux-gnu.tgz --output myceliald-aarch64-unknown-linux-gnu.tgz
   tar -xvzf myceliald-aarch64-unknown-linux-gnu.tgz
   ```

   </details>



### Configure the client

Mycelial clients require a configuration file to function properly.

You can find a [config.example.toml](myceliald/config.example.toml) example file,
which you can reference when creating your own client configuration files.

The configuration file is passed to the client binary file via the
`./myceliald --config=./config.toml` command line option. 

Copy the [config.example.toml](myceliald/config.example.toml), rename it to 
`config.toml`.

Edit the copied config file as follows. **NOTE** be sure to change the 
`journal_path` to point to the journal file created ealier.

```toml
[node]
display_name = "Client 1"
unique_id = "client_1"
storage_path = "client.sqlite"

[server]
endpoint = "http://localhost:8080" 
token = "secrettoken" 

[[sources]]
type = "sqlite_physical_replication"
display_name = "Example Source"
journal_path = "{full path and file name to the mycelite journal}"

[[destinations]]
type = "sqlite_physical_replication"
display_name = "Sqlite Physical Replication Movie"
journal_path = "/tmp/destination.sqlite.mycelial"
database_path = "/tmp/destination.sqlite"
```

### Start the client

After you've configured the client, start the client.

```sh
./myceliald --config ./config.toml
```

## Create a data pipeline

Open your web browser and navigate to `http://localhost:8080`. When prompted
for a username and password, enter the token `secrettoken` into the username
field.

In the upper left portion of the page, you'll see the source and destination 
that we configured for the client, and the `Mycelial Server`.

Drag and drop the source node onto the canvas.

Next, drag and drop the `Mycelial Server` onto the canvas. Set the token to 
`secrettoken`.

Next, drag and drop the destination node onto the canvas.

Lastly, connect the source to the destination, via the mycelial network.

Now push the `publish` button to start the synchronization workflow.

## Observe the synchronization

At this point, you can navigate to `/tmp/` and you'll see a `destination.sqlite`
SQLite database.

Open and query the database as follows:

```sh
cd /tmp
sqlite3 destination.sqlite
sqlite> .tables # shows database tables
users
sqlite> SELECT * FROM USERS; # you should see the user named 'John'
id  name
--  ----
1   John
```

