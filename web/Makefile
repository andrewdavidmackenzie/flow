build:
	@echo ""
	@echo "------- Started building 'web' module -------------"
	@echo "------- Building for 'wasm32' target -------------"
	cargo +nightly build --target wasm32-unknown-unknown
	@echo "------- Generating Javascript bindings using 'wasm-bindgen' -------------"
	wasm-bindgen target/wasm32-unknown-unknown/debug/web.wasm --out-dir run
	@echo "------- Finished building 'web' module -------------"

run: build
	cd run && npm install
	cd run && npm run serve

test:
	cargo test

clean:
	cargo clean
	rm -rf run/node_modules
	rm -f run/web.js run/web.d.ts run/web_bg.js run/web_bg.d.ts run/web_bg.wasm