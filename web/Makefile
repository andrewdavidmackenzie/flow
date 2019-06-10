build:
	@echo ""
	@echo "--- Started building 'web' module for 'wasm32' target --------"
	@cargo build --target wasm32-unknown-unknown
	@echo "--- Generating Javascript bindings using 'wasm-bindgen' ------"
	@wasm-bindgen target/wasm32-unknown-unknown/debug/web.wasm --out-dir run
	@echo "------------ Finished building 'web' module ------------------\n"

run: build
	cd run && npm install
	cd run && npm run serve

test:
	@echo "--- Started  'cargo test' for 'web' module on native target ---"
	@cargo test
	@echo "--- Finished 'cargo test' for 'web' module on native target ---"

clean:
	cargo clean
	rm -rf run/node_modules
	rm -f run/web.js run/web.d.ts run/web_bg.js run/web_bg.d.ts run/web_bg.wasm