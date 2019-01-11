build: native wasm

native:
	@echo ""
	@echo "------- Starting build of 'flowstdlib' native -------------"
	cargo build
	@echo "------- Done     build of 'flowstdlib' native -------------"

wasm:
	@echo ""
	@echo "------- Starting build of 'flowstdlib' wasm -------------"
	cargo build --target=wasm32-unknown-unknown
	@echo "------- Done     build of 'flowstdlib' wasm -------------"

test: test-native # test-wasm

test-native:
	@echo ""
	@echo "------- Starting test of 'flowstdlib' native -------------"
	cargo test
	@echo "------- Done     test of 'flowstdlib' native -------------"

clean:
	cargo clean