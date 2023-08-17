# mycelial

## Local dev setup
1. Setup rust: use provided script at https://rustup.rs/ (Follow [these instructions](https://stackoverflow.com/questions/67656028/rustup-gives-command-not-found-error-with-zsh-even-after-installing-with-brew) if it doesn't work initially.)
2. Update rust: `rustup update`
3. Install brew (assuming mac os): use provided script at https://brew.sh/
4. Install node: run `brew install node`
5. Install `cargo-watch` tool: `cargo install cargo-watch`

### How to run services:
To run in dev mode you need to launch server, client and ui (`myc-console`).  
Each folder contains makefile, to run in dev mode - execute `make dev`.
1. Navigate to `mycelial/server`, run `make dev`
1. Navigate to `mycelial/client`, run `make dev`
1. Navigate to `mycelial/myc-console`, run `make dev`
1. Go to `localhost:8080` in your browser
  1. You may need to execute `npm run build` in `mycelial/myc-console` and restart the server, client, and ui if port :8080 isn't forwarding to your server
Both server and client are using `cargo-watch` tool, which could be installed via `cargo install cargo-watch`.


### Troubleshooting:

If you use SSH to authenticate to GitHub and get an error complaining about authenticating to GitHub using HTTPS when running `make dev`, try running the following:
```
$ git config --global url.ssh://git@github.com/.insteadOf https://github.com/
```
