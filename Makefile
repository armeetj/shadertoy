.PHONY: dev build clean install check deploy

dev:
	trunk serve --open

build:
	trunk build --release --dist dist

# Build for GitHub Pages — set PUBLIC_URL to your repo name
# e.g. make deploy PUBLIC_URL=/glsl-notebook/
PUBLIC_URL ?= /
deploy:
	trunk build --release --dist dist --public-url $(PUBLIC_URL)

clean:
	cargo clean
	rm -rf dist

check:
	cargo check

install:
	cargo install trunk wasm-bindgen-cli
	rustup target add wasm32-unknown-unknown
