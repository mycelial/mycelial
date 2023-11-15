## Running Mycelial in development mode

The following instructions will start Mycelial in development mode. 

### Setup development environment

1. Setup Rust: use provided script at [rustup.rs](https://rustup.rs)
2. Update Rust: `rustup update`
3. Install cargo-watch with `cargo install cargo-watch`
4. Install [Nodejs](https://nodejs.org/en/download) >= v18
5. Clone the repo `git clone git@github.com:mycelial/mycelial.git`

### Mycelial Server

1. Navigate to `mycelial/myceliald`
2. Build the frontend with `make build`
3. Navigate to `mycelial/server`
4. Start the server with `make dev`

### Mycelial Client (myceliald)

1. Navigate to `mycelial/myceliald`
5. Run `make dev` to start the client

At this point you should be able to open the web [interface](http://localhost:7777)

**NOTE**: You can stop the server and client by pressing `ctrl-c` in the
associated terminals.
