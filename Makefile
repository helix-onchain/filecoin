install-toolchain:
	rustup component add rustfmt
	rustup component add clippy
	rustup target add wasm32-unknown-unknown

build: install-toolchain
	cargo build --workspace

check: install-toolchain
	cargo fmt --check
	cargo clippy --workspace -- -D warnings

check-build: check
	cargo build --workspace

test: install-toolchain
	cargo test

clean:
	cargo clean
	find . -name '*.profraw' -delete
