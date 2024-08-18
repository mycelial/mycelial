RUST_LOG ?= info

.PHONY: dev
dev:
	cargo-watch -q -c -s 'make run_dev'


.PHONY: run_dev
run_dev: build_ui run_control_plane


.PHONY: build_ui
build_ui: 
	cd ui && dx build


.PHONY: run_console
run_control_plane:
	RUST_LOG=$(RUST_LOG)  cargo run \
		--bin control_plane \
		-- \
		--database-url sqlite://control_plane/control_plane.db

.PHONY: fmt
fmt:
	cargo fmt && cargo clippy --fix --allow-dirty --allow-staged
