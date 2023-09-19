# Mycelial

Out of necessity, Edge Machine Learning (ML) moves computing away from the data
center to the edges of the network where the data is generated. This means your
resource-intensive ML applications must be local applications to function
properly.

This raises an important question, which is: 

> "How do you access the important information on your edge devices?"

You could spend engineering time trying to synchronize the data on your edge 
devices with your data center, but this is a hard problem to solve, and you can
save yourself a lot of time and effort by using Mycelial to move your data where it needs to go.

Mycelial offers you an easy solution to your data movement needs.

## What is Mycelial?

Mycelial is an open-source software solution that moves data from sources to destinations. 

For example, consider an Edge Machine Learning application that stores its
information in a local [SQLite](https://sqlite.org/) database.

So, how do you get the information off of your edge device and onto a system
where the information can be analyzed?

Well, with Mycelial you can declaratively create data pipelines that move your 
data from a source system like SQLite, to a destination system like
[Snowflake](https://www.snowflake.com/).

## How does it work?

There are two main components in Mycelial: clients and a server.

The client is installed and executed on source and destination computers. These
clients will register with the server component, and they will receive
data pipeline specifications from the server.

The server offers you a way to set up data pipeline specifications, which move
your data from one location to another via the installed clients. You can set up
these data pipelines via a drag-and-drop web interface, or you can add these
pipelines via api calls.

Once you've installed Mycelial (clients and server) you can easily begin moving
your data from source systems to destination systems of your choosing. Currently
Mycelial has adapters for SQLite but many other adapters are in the works. 

## How do I get started?

Follow our [Setup Instructions](/docs/Setup.md) to install clients and servers.

## How do I move my data from A to B?

Follow our [getting started](/docs/Getting_Started.md) guides which walk you
through how to set up data pipelines.

## API

API [documentation](/docs/API.md)

## Community

- [Discord](https://discord.gg/mycelial)
- [@mycelial](https://twitter.com/mycelial)
- [Newsletter](https://mycelial.com/#newsletter)

## License

Mycelial is available under the [Apache 2 license](LICENSE).