# Getting Started

Follow the [Setup](./Setup.md) instructions to install and start the Mycelial
server and clients.

## Opening the web interface

When you open up the [web interface](http://localhost:8080) you'll be prompted
with a Basic Authentication modal window. Enter the token you specified in the
`--token` option when starting the server.

## Available nodes

In the upper left corner of the web page, you'll see the sources and
destinations that the client(s) have made available to you. If you don't see a 
source or destination that you wish to use, you'll need to modify the clients
[toml](../client/config.example.toml) file to include the sources and/or
destinations you wish to use.

## SQLite to SQLite replication with Mycelite

[Mycelite](https://github.com/mycelial/mycelite) used in conjunction with
Mycelial allows you to fully replicate a SQLite database from a read/write
source to a readonly destination.

To setup replication do the following:

1. [Download](https://mycelial.com/docs/get-started/quick-start/#download-the-extension) the Mycelite extension to the computer with the source database.
2. [Load](https://mycelial.com/docs/get-started/quick-start/#load-the-extension) the extension in your code
3. [Open](https://mycelial.com/docs/get-started/quick-start/#open-a-new-database-as-a-writer) your database and begin using.

After you've loaded the extension, Mycelite will create a journal file, next to
your SQLite database file. The journal file is used by Mycelite to replicate the
database.

### Create SQLite to SQLite data pipeline in the web interface

1. Drag and drop your source node onto the canvas.
2. Drag and drop your destination node onto the canvas.
3. Connect the source and destination nodes
4. Publish your configuration

At this point, any changes you make in the source database will be replicated to
the destination database.

### Create SQLite to SQLite data pipeline via API call

If you prefer to setup your data pipelines with an API call, do the following.

Create the pipeline by making a `POST` configuration api call to
`/api/pipe/configs` wit the following payload:

```json
{
  "configs": [
    {
      "id": 0,
      "pipe": [
        {
          "name": "mycelite_source",
          "client": "{client name}",
          "journal_path": "{path and filename of source journal}"
        },
        {
          "name": "mycelite_destination",
          "client": "{client name}",
          "journal_path": "{path and file name of destination journal}",
          "database_path": "{path and file name of destination database}"
        }
      ]
    }
  ]
}

```