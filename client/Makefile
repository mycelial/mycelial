SHELL := bash
SQLITE_PATH ?= "/tmp/test.sqlite"
TAG ?= pipe_client

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
	rm -f dev_client.sqlite*

.PHONY: dev
dev:
	cargo-watch -c -q \
		-i dev_client.sqlite \
		-i dev_client.sqlite-journal \
		-s 'cargo run -r -- --endpoint="http://localhost:8080" --token=token --name="dev_client" --storage-path="dev_client.sqlite"'

.PHONY: vendor
vendor:
	cargo vendor > build/docker_cargo_config.toml

.PHONY: dockerize_vendored
dockerize_vendored:
	cd ../ && docker build -f client/Dockerfile -t $(TAG) .


.PHONY: run_docker
run_docker:
	docker run -d --rm --name $(TAG) -p 0.0.0.0:8080:8080 $(TAG) 


.PHONY: stop_docker
stop_docker:
	docker rm -f $(TAG)

.PHONY: deploy_at_flyio
deploy_at_flyio:
	cd ../ && flyctl deploy -c client/flyio.toml

.PHONY: attach_to_flyio
attach_to_flyio:
	flyctl -c flyio.toml ssh console
