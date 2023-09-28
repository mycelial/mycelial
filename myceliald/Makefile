SHELL := bash
SQLITE_PATH ?= "/tmp/test.sqlite"
TAG ?= myceliald
RUST_LOG ?= INFO

.PHONY: all
all:

.PHONY: bootstrap_sqlite
bootstrap_sqlite:
	rm -f $(SQLITE_PATH)
	echo "SELECT * FROM test" | sqlite3 $(SQLITE_PATH) \
		-cmd 'CREATE TABLE test (id INT PRIMARY KEY, text TEXT, bin BLOB)' \
		-cmd 'INSERT OR IGNORE INTO test VALUES(1, "foo", "foo"), (2, "bar", "bar"), (3, NULL, NULL)' \
		-cmd '.mode table'

.PHONY: clean
clean:
	rm -f myceliald_state.sqlite*

.PHONY: dev
dev:
	RUST_LOG=$(RUST_LOG) cargo-watch -c -q \
		-i myceliald_state.sqlite \
		-i myceliald_state.sqlite-journal \
		-s 'cargo run -- --config=config.example.toml'

.PHONY: vendor
vendor:
	cargo vendor > build/docker_cargo_config.toml

.PHONY: dockerize_vendored
dockerize_vendored:
	cd ../ && docker build -f myceliald/Dockerfile -t $(TAG) .


.PHONY: run_docker
run_docker:
	docker run -d --rm --name $(TAG) -p 0.0.0.0:8080:8080 $(TAG) 


.PHONY: stop_docker
stop_docker:
	docker rm -f $(TAG)
