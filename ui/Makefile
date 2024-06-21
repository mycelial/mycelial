.PHONY: dev
dev:
	dx serve --hot-reload

.PHONY: watch-tailwind
watch-tailwind:
	npx tailwind -i tailwind/input.css -c tailwind/tailwind.config.js -o assets/tailwind.css --watch -m

.PHONY: build
build:
	npx tailwind -i tailwind/input.css -c tailwind/tailwind.config.js -o assets/tailwind.css -m
	dx build -r
