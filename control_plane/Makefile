.PHONY: dev
dev:
	$(MAKE) -C ../ui build
	cargo-watch \
		-q \
		-c \
		--why \
		-i 'control_plane.db' \
		-i 'control_plane.db-journal' \
		-s 'cargo run'
