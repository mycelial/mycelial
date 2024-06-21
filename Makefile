.PHONY: dev
dev:
	cargo-watch -q -c \
		-s 'dx build --bin ui' \
		-s 'cargo run --bin control_plane -- --database-url sqlite://control_plane/control_plane.db'


.PHONY: fmt
fmt:
	cargo fmt && cargo clippy --fix --allow-dirty --allow-staged
