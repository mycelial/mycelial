RUST_LOG ?= INFO

.PHONY: all
all:

.PHONY: clean
clean:
	rm -f myceliald.db

.PHONY: dev
dev:
	cargo-watch \
		--why \
		-E RUST_LOG="$(RUST_LOG)" \
		-i myceliald.db \
		-i myceliald.db-journal \
		-s 'cargo run'
