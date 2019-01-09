build:
	@echo ""
	@echo "------- Building flowrlib native -------------"
	cargo build
	@echo "------- Building flowrlib wasm32 -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     building flowrlib -------------"

test: build
	cargo test

clean:
	cargo clean