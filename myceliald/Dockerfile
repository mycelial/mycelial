# myceliald
FROM rust:alpine AS build_myceliald

WORKDIR /app/
COPY . /app/

RUN apk add musl-dev openssl-dev bash build-base && \
    cd /app/myceliald/ && \
    RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release


# final image
FROM alpine:3.18
RUN apk add --no-cache libgcc sqlite

COPY --from=build_myceliald /app/target/release/myceliald /app/
COPY --from=build_myceliald /app/myceliald/config.flyio.toml /app/

RUN echo "SELECT * FROM test" | sqlite3 /tmp/sqlite.db \
		-cmd 'CREATE TABLE test (key INT PRIMARY KEY, text TEXT, bin BLOB)' \
		-cmd 'INSERT INTO test VALUES(1, "foo", "foo"), (2, "bar", "bar"), (NULL, NULL, NULL)' \
		-cmd '.mode table'

WORKDIR /app
ENTRYPOINT /app/myceliald
