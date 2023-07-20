SHELL := bash
TAG ?= pipe_server
export ASSETS_DIR ?= $(shell echo `dirname $(PWD)`/myc-console/out/)

.PHONY: all
all:


.PHONY: dev
dev:
	cargo-watch -q -c -s 'cargo run -- --token="foobar"'


.PHONY: vendor
vendor:
	cargo vendor 2>/dev/null > build/docker_cargo_config.toml

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
