build: build-wasm

build-wasm:
	@echo ""
	@echo "------- Starting build of 'web-provider' wasm -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     build of 'web-provider' wasm -------------"

test: test-native build-wasm

test-native:
	@echo ""
	@echo "------- Starting test of 'web-provider' native -------------"
	cargo test
	@echo "------- Done     test of 'web-provider' native -------------"

clean:
	cargo clean