# Mycelial Setup

This document will guide you through the setup steps to start using Mycelial. If
you're using Mycelial for the first time, consider using this
[tutorial](./Tutorial.md) first.

To use Mycelial, you will need to:
1. Start the Mycelial server
2. Configure & start the Mycelial client(s) (myceliald)

## Running Mycelial using binaries

### Mycelial Server

1. Download and unarchive the server binary for your system

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

2. Start the server by executing the server binary passing along a security token. ex: `./server --token token`
   **NOTE**: Use a secure, secret token in production

### Mycelial Client (myceliald) Setup

1. Download and unarchive the binary for your system

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

2. Make a copy of [config.example.toml](../myceliald/config.example.toml) and name it `config.toml`
3. Modify the `config.toml` file as follows:
   1. Under the `[node]` table, modify the `display_name` and the `unique_id` as desired
   2. Under the `[server]` table, modify the `endpoint` value to be the servers address and set the `token` to be the security token you used when starting the server.
   3. Add the [sources and destinations](./Client_Configuration.md) that you'd like for the client
4. Execute the client with the `--config` option passing along the `config.toml` file. ex: `./myceliald --config ./config.toml`

At this point you should be able to open the web [interface](http://localhost:8080)

## Running Mycelial in development mode

The following instructions will start Mycelial in development mode. 

### Setup development environment

1. Setup Rust: use provided script at [rustup.rs](https://rustup.rs)
2. Update Rust: `rustup update`
3. Install cargo-watch with `cargo install cargo-watch`
4. Install [Nodejs](https://nodejs.org/en/download) >= v18

### Mycelial Server

1. Navigate to `mycelial/console`
2. Build the frontend with `make build`
3. Navigate to `mycelial/server`
4. Start the server with `make dev`

### Mycelial Client (myceliald)

1. Navigate to `mycelial/myceliald`
5. Run `make dev` to start the client

At this point you should be able to open the web [interface](http://localhost:8080)

**NOTE**: You can stop the server and client by pressing `ctrl-c` in the
associated terminals.