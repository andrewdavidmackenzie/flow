build: native wasm

native:
	@echo ""
	@echo "------- Starting build of 'flowclib' native -------------"
	cargo build
	@echo "------- Done     build of 'flowclib' native -------------"

wasm:
	@echo "------- Starting build of 'flowclib' wasm -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     build of 'flowclib' wasm -------------"

test: test-native wasm

test-native:
	@echo "------- Starting test of 'flowclib' native -------------"
	cargo test
	@echo "------- Done     test of 'flowclib' native -------------"

clean:
	cargo clean
