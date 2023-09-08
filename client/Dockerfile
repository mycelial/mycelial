# pipe client
FROM rust:alpine AS build_client

WORKDIR /app/client
COPY client/ /app/client/
COPY common /app/common/
COPY Cargo.lock /app/client/
COPY client/build/docker_cargo_config.toml /app/client/.cargo/config.toml

RUN apk add musl-dev openssl-dev bash build-base && \
    RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release


# final image
FROM alpine:3.18
RUN apk add --no-cache libgcc sqlite

COPY --from=build_client /app/client/target/release/client /app/client
COPY --from=build_client /app/client/config.flyio.toml /app/client

RUN echo "SELECT * FROM test" | sqlite3 /tmp/sqlite.db \
		-cmd 'CREATE TABLE test (key INT PRIMARY KEY, text TEXT, bin BLOB)' \
		-cmd 'INSERT INTO test VALUES(1, "foo", "foo"), (2, "bar", "bar"), (NULL, NULL, NULL)' \
		-cmd '.mode table'

WORKDIR /app
ENTRYPOINT /app/client
