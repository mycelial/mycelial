# Mycelial Setup

You must either download the binaries or clone Mycelial to all clients and the
server.

The source and destination computers need a Mycelial client(myceliald)
installed.

## Mycelial Client (myceliald) Setup

###  Using myceliald binary

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
   2. Under the `[server]` table, modify the `endpoint` value to be the servers address and set the `token` to be the security token you intend on using on the server.
   3. Add the sources and destinations that you'd like for the client
4. Execute the client with the `--config` option passing along the `config.toml` file. ex: `./myceliald --config ./config.toml`

### Using myceliald in development

1. Setup Rust: use provided script at [rustup.rs](https://rustup.rs)
2. Update Rust: `rustup update`
3. Install cargo-watch with `cargo install cargo-watch`
4. Navigate to `mycelial/myceliald`
5. Make a copy of [config.example.toml](../myceliald/config.example.toml) and name the copy `config.toml`.
6. Modify the `config.toml` file as follows:
   1. Under the `[node]` table, modify the `display_name` and the `unique_id` as desired
   2. Under the `[server]` table, modify the `endpoint` value to be the servers address and set the `token` to be the security token you intend on using on the server.
   3. Add the sources and destinations that you'd like for the client
7.  Modify the Makefile's `dev` rule, change the `--config` option to point to your new `config.toml` file
8.  run `make dev` to start the client

## Mycelial Server

### Using the mycelial binary

1. Download and unarchive the binary for your system

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

2. Execute the server binary passing along a security token. ex: `./server --token MySecretToken`

### Using mycelial server in development

1. Navigate to `mycelial/server`
2. Modify the [Makefile](../server/Makefile.md), changing the `--token` options value to the security token you wish to use. This token should match the tokens used by the clients.
3. Start the server with `make dev`

## Mycelial Console in development

1. Navigate to `mycelial/console`
2. Build the frontend with `make build`

At this point you should be able to open the web [interface](http://localhost:8080)
