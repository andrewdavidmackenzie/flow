build: build-wasm

build-wasm:
	@echo ""
	@echo "------- Starting build of 'nodeprovider' wasm -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     build of 'nodeprovider' wasm -------------"

test:
	@echo ""
	@echo "--- Started  'cargo build' for 'nodeprovider' on wasm target ---"
	@cargo build --target wasm32-unknown-unknown
	@echo "--- Ended    'cargo build' for 'nodeprovider' on wasm target ---"
	@echo ""
	@echo "--- Started  'cargo test' for 'nodeprovider' on native target ---"
	@cargo test
	@echo "--- Finished 'cargo test' for 'web/crate' on native target ---"

clean:
	cargo clean