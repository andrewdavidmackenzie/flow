build:
	@echo ""
	@echo "------- Building flowclib native -------------"
	cargo build
	@echo "------- Building flowclib wasm32 -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     building flowrlib -------------"

test: build
	cargo test

clean:
	cargo clean