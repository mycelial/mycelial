# myceliald
FROM rust:alpine AS build_myceliald

WORKDIR /app/myceliald
COPY myceliald/ /app/myceliald/
COPY common /app/common/
COPY pipe /app/pipe/
COPY Cargo.lock /app/myceliald/
COPY myceliald/build/docker_cargo_config.toml /app/myceliald/.cargo/config.toml

RUN apk add musl-dev openssl-dev bash build-base && \
    RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release


# final image
FROM alpine:3.18
RUN apk add --no-cache libgcc sqlite

COPY --from=build_myceliald /app/myceliald/target/release/myceliald /app/myceliald
COPY --from=build_myceliald /app/myceliald/config.flyio.toml /app/myceliald

RUN echo "SELECT * FROM test" | sqlite3 /tmp/sqlite.db \
		-cmd 'CREATE TABLE test (key INT PRIMARY KEY, text TEXT, bin BLOB)' \
		-cmd 'INSERT INTO test VALUES(1, "foo", "foo"), (2, "bar", "bar"), (NULL, NULL, NULL)' \
		-cmd '.mode table'

WORKDIR /app
ENTRYPOINT /app/myceliald
