build:
	cargo build --target wasm32-unknown-unknown
	wasm-bindgen target/wasm32-unknown-unknown/debug/web.wasm --out-dir .

run: build
	npm install
	npm run serve

clean:
	rm -rf target
	rm -rf web.d.ts web.js web_bg.d.ts web_bg.wasm