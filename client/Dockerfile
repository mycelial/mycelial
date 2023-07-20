# pipe client
FROM rust:alpine AS build_client

WORKDIR /app
COPY client/ /app
COPY Cargo.lock /app/
COPY client/build/docker_cargo_config.toml /app/.cargo/config.toml

RUN apk add musl-dev openssl-dev bash build-base && \
    RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release


# final image
FROM alpine:3.18
RUN apk add --no-cache libgcc

COPY --from=build_client /app/target/release/client /app/client

WORKDIR /app
ENTRYPOINT /app/client
