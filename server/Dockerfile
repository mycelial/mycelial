# frontend assets
FROM node:lts-alpine3.18 as build_frontend_assets
WORKDIR /app
COPY console/ /app/
RUN npm i && npm run build

# server
FROM rust:alpine AS build_server

WORKDIR /app/
COPY . /app/
COPY --from=build_frontend_assets /app/out/ /app/console/out/
RUN apk add musl-dev openssl-dev bash build-base && \
    cd /app/server && \
    RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release

# final image
FROM alpine:3.18
RUN apk add --no-cache libgcc

COPY --from=build_server /app/target/release/server /app/server

WORKDIR /app
EXPOSE 8080
ENTRYPOINT /app/server
