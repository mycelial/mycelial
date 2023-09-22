# Mycelial Setup

You must clone Mycelial to all clients and the server.

The source and destination computers need a Mycelial client installed.

## Mycelial Client Setup

1. Setup Rust: use provided script at [rustup.rs](https://rustup.rs)
2. Update Rust: `rustup update`
3. Install cargo-watch with `cargo install cargo-watch`
4. Navigate to `mycelial/client`
5. Make a copy of [config.example.toml](../client/config.example.toml) and name the copy `config.toml`.
6. Modify the `config.toml` file as follows:
   1. Under the `[node]` table, modify the `display_name` and the `unique_id` as desired
   2. Under the `[server]` table, modify the `endpoint` value to be the servers address and set the `token` to be the security token you intend on using on the server.
   3. Add the sources and destinations that you'd like for the client
7. Modify the Makefile's `dev` rule, change the `--config` option to point to your new `config.toml` file
8. run `make dev` to start the client

## Mycelial Server

1. Navigate to `mycelial/server`
2. Modify the [Makefile](../server/Makefile.md), changing the `--token` options value to the security token you wish to use. This token should match the tokens used by the clients.
3. Start the server with `make dev`

## Mycelial Webserver

1. Navigate to `mycelial/console`
2. Build the frontend with `make build`

At this point you should be able to open the web [interface](http://localhost:8080)