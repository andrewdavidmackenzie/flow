build: build-wasm

build-wasm:
	@echo ""
	@echo "------- Starting build of 'webprovider' wasm -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     build of 'webprovider' wasm -------------"

test:
	@echo ""
	@echo "--- Started  'cargo build' for 'webprovider' on wasm target ---"
	@cargo build --target wasm32-unknown-unknown
	@echo "--- Ended    'cargo build' for 'webprovider' on wasm target ---"
	@echo ""
	@echo "--- Started  'cargo test' for 'webprovider' on native target ---"
	@cd crate;cargo test
	@echo "--- Finished 'cargo test' for 'web/crate' on native target ---"

clean:
	cargo clean