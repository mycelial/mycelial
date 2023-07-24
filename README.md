# mycelial

## Local dev setup
1. Setup rust: use provided script at https://rustup.rs/
2. Update rust: `rustup update`
3. Install brew (assuming mac os): use provided script at https://brew.sh/
4. Install node: run `brew install node`
5. Install `cargo-watch` tool: `cargo install cargo-watch`

### How to run services:
To run in dev mode you need to launch server, client and ui.  
Each folder contains makefile, to run in dev mode - execute `make dev`.
Both server and client are using `cargo-watch` tool, which could be installed via `cargo install cargo-watch`.

### Troubleshooting:

If you use SSH to authenticate to GitHub and get an error complaining about authenticating to GitHub using HTTPS when running `make dev`, try running the following:
```
$ git config --global url.ssh://git@github.com/.insteadOf https://github.com/
```
