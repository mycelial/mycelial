# Getting Started

Follow the [Setup](./Setup.md) instructions to install and start the Mycelial
server and clients.

Follow this [Tutorial](./Tutorial.md) if you are using Mycelial for the first
time.

## Opening the web interface

When you open up the [web interface](http://localhost:8080) you'll be prompted
with a Basic Authentication modal window. Enter the token you specified in the
`--token` option when starting the server. **Note** the token goes in the
_username_ field.

## Available nodes

In the upper left corner of the web page, you'll see the sources and
destinations that the client(s) have made available to you. If you don't see a 
source or destination that you wish to use, you'll need to modify the clients
[toml](../myceliald/config.example.toml) file to include the sources and/or
destinations you wish to use.
