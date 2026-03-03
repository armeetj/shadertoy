.PHONY: dev build clean install check

dev:
	trunk serve --open

build:
	trunk build --release --dist dist

clean:
	cargo clean
	rm -rf dist

check:
	cargo check

install:
	cargo install trunk wasm-bindgen-cli
	rustup target add wasm32-unknown-unknown
