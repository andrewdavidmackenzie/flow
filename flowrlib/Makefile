build: native wasm

native:
	@echo ""
	@echo "------- Starting build of 'flowrlib' native -------------"
	cargo build
	@echo "------- Done     build of 'flowrlib' native -------------"

wasm:
	@echo ""
	@echo "------- Starting build of 'flowrlib' wasm -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     build of 'flowrlib' wasm -------------"

test: test-native # test-wasm

test-native:
	cargo test

test-wasm:
	cargo test --target=wasm32-unknown-unknown

clean:
	cargo clean