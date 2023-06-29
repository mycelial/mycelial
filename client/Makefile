.PHONY: all
all:

.PHONY: bootstrap_sqlite
bootstrap_sqlite:
	rm -f test.sqlite
	echo "SELECT * FROM test" | sqlite3 test.sqlite \
		-cmd 'CREATE TABLE test (key INT PRIMARY KEY, text TEXT, bin BLOB)' \
		-cmd 'INSERT INTO test VALUES(1, "foo", "foo"), (2, "bar", "bar"), (NULL, NULL, NULL)' \
		-cmd '.mode table'

.PHONY: dev
dev:
	cargo-watch -c -q -s 'cargo run -r -- --endpoint="http://localhost:8080/ingestion" --token="foobar" --sqlite-path="./test.sqlite"'
