build:
	cargo build --target wasm32-unknown-unknown
	wasm-bindgen target/wasm32-unknown-unknown/debug/web.wasm --out-dir run

run: build
	cd run && npm install
	cd run && npm run serve

test: clean build

clean:
	rm -rf target
	rm -rf run/node_modules
	rm -f run/web.js run/web.d.ts run/web_bg.js run/web_bg.d.ts run/web_bg.wasm