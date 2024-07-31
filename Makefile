RUST_LOG ?= info

.PHONY: dev
dev:
	cargo-watch -q -c \
		-E RUST_LOG=$(RUST_LOG) \
		-s 'export RUSTFLAGS=--cfg=web_sys_unstable_apis && dx build --bin ui' \
		-s 'cargo run \
			--bin control_plane \
			-- \
			--database-url sqlite://control_plane/control_plane.db'


.PHONY: fmt
fmt:
	cargo fmt && cargo clippy --fix --allow-dirty --allow-staged
