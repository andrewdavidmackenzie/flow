build: build-wasm

build-wasm:
	@echo ""
	@echo "------- Starting build of 'webprovider' wasm -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     build of 'webprovider' wasm -------------"

test: test-native build-wasm

test-native:
	@echo ""
	@echo "------- Starting test of 'webprovider' native -------------"
	cargo test
	@echo "------- Done     test of 'webprovider' native -------------"

clean:
	cargo clean