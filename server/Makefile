SHELL := bash
TAG ?= pipe_server
export ASSETS_DIR ?= $(shell echo `dirname $(PWD)`/myc-console/out/)

.PHONY: all
all:

mycelial.db:
	sqlite3 mycelial.db "VACUUM;"
	sqlx migrate run --database-url sqlite://mycelial.db

.PHONY: clean
clean:
	rm -f mycelial.db*

.PHONY: dev
dev:
	RUST_LOG=DEBUG cargo-watch -q -c \
		-i mycelial.db \
		-s 'cargo run -- --token=token'

.PHONY: vendor
vendor:
	cargo vendor > build/docker_cargo_config.toml

.PHONY: dockerize_vendored
dockerize_vendored:
	cd ../ && docker build -f server/Dockerfile -t $(TAG) .

.PHONY: run_docker
run_docker:
	docker run -d --rm --name $(TAG) -p 0.0.0.0:8080:8080 $(TAG) 


.PHONY: stop_docker
stop_docker:
	docker rm -f $(TAG)

.PHONY: deploy_at_flyio
deploy_at_flyio:
	cd ../ && flyctl deploy -c server/flyio.toml


.PHONY: attach_at_flyio
attach_at_flyio:
	flyctl -c flyio.toml ssh console
