build: native # wasm

native:
	@echo ""
	@echo "------- Starting build of 'flowclilib' native -------------"
	cargo build
	@echo "------- Done     build of 'flowclilib' native -------------"

wasm:
	@echo ""
	@echo "------- Starting build of 'flowclilib' wasm -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     build of 'flowclilib' wasm -------------"

test: test-native # test-wasm

test-native:
	cargo test

test-wasm:
	cargo test --target=wasm32-unknown-unknown

clean:
	cargo clean