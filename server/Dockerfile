# pipe server
FROM rust:alpine AS build_server

WORKDIR /app/server
COPY server/ /app/server/
COPY common/ /app/common/
COPY Cargo.lock /app/server/
COPY server/build/docker_cargo_config.toml /app/server/.cargo/config.toml

RUN apk add musl-dev openssl-dev bash && \
    RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release


# frontend assets
FROM node:lts-alpine3.18 as build_frontend_assets
WORKDIR /app
COPY myc-console/ /app/
RUN npm i && npm run build


# final image
FROM alpine:3.18
RUN apk add --no-cache libgcc

COPY --from=build_server /app/server/target/release/server /app/server
COPY --from=build_frontend_assets /app/out/ /app/assets/

WORKDIR /app
EXPOSE 8080
ENV ASSETS_DIR=/app/assets/
ENTRYPOINT /app/server
