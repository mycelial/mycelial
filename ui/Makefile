.PHONY: dev
dev:
	dx serve --hot-reload

.PHONY: watch-tailwind
watch-tailwind:
	npx tailwind -i tailwind/input.css -c tailwind/tailwind.config.js -o assets/tailwind.css --watch -m

.PHONY: build
build: export RUSTFLAGS=--cfg=web_sys_unstable_apis  
build:
	npx tailwind -i tailwind/input.css -c tailwind/tailwind.config.js -o assets/tailwind.css -m
	dx build -r
