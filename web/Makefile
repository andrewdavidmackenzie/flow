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
	@echo "--- Started  'cargo test' for 'web/crate' on native target ---"
	@cd crate;cargo test
	@echo "--- Finished 'cargo test' for 'web/crate' on native target ---"

clean:
	cargo clean
	rm -rf crate/bin crate/pkg/* crate/target crate/Cargo.lock crate/wasm-pack.log dist .bin