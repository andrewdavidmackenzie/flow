build:
	@echo ""
	@echo "--- Started building 'web' module for 'wasm32' target --------"
	@npm install
	@npm run build
	@echo "------------ Finished building 'web' module ------------------\n"

run: build
	npm start

test:
	@echo "--- Started  'cargo test' for 'web' module on native target ---"
	@cargo test
	@echo "--- Finished 'cargo test' for 'web' module on native target ---"

clean:
	cargo clean
	rm -rf crate/bin crate/pkg crate/target crate/Cargo.lock crate/wasm-pack.log dist .bin crate/.crates.toml