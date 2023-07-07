SQLITE_PATH ?= "/tmp/test.sqlite"

.PHONY: all
all:

.PHONY: bootstrap_sqlite
bootstrap_sqlite:
	rm -f $(SQLITE_PATH)
	echo "SELECT * FROM test" | sqlite3 $(SQLITE_PATH) \
		-cmd 'CREATE TABLE test (key INT PRIMARY KEY, text TEXT, bin BLOB)' \
		-cmd 'INSERT INTO test VALUES(1, "foo", "foo"), (2, "bar", "bar"), (NULL, NULL, NULL)' \
		-cmd '.mode table'

.PHONY: dev
dev: bootstrap_sqlite
	cargo-watch -c -q \
		-i test.sqlite \
		-s 'cargo run -- --endpoint="http://localhost:8080" --token="foobar"'
