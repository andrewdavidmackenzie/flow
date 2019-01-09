build:
	@echo ""
	@echo "------- Building flowstdlib native -------------"
	cargo build
	@echo "------- Building flowstdlib wasm32 -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     building flowstdlib -------------"

test: build
	cargo test

clean:
	cargo clean