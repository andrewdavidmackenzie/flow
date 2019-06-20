build:
	@echo ""
	@echo "--- Started building 'web' module for 'wasm32' target --------"
	@npm install
	wasm-pack build crate/
	@npm run build
	@echo "------------ Finished building 'web' module ------------------\n"

run: build
	@echo ""
	@echo "--- Starting 'web' module via npm--------"
	@npm start

test:
	@echo ""
	@echo "--- Started  'cargo build' for 'web/crate' on wasm target ---"
	@cd crate;cargo build --target wasm32-unknown-unknown
	@echo "--- Ended    'cargo build' for 'web/crate' on wasm target ---"
	@echo ""
	@echo "--- Started  'cargo test' for 'web/crate' on native target ---"
	@cd crate;cargo test
ifeq ($(TRAVIS),)
	@cd crate;wasm-pack test --chrome --headless -- --features wasm-bindgen
endif
	@echo "--- Finished 'cargo test' for 'web/crate' on native target ---"

clean:
	cargo clean
	rm -rf crate/bin crate/pkg/* crate/target crate/Cargo.lock crate/wasm-pack.l	og dist .bin