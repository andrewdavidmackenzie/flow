run: wasm
	electron-forge start	

wasm: src/flowui.wasm src/flowui.js

src/flowui.wasm: target/wasm32-unknown-emscripten/release/deps
	@find target/wasm32-unknown-emscripten/release/deps -type f -name "*.wasm" | xargs -I {} cp {} $@
	@echo "emscripten wasm files updated"

src/flowui.js: target/wasm32-unknown-emscripten/release/deps
	@cp src/electron-prefix.js $@
	@find target/wasm32-unknown-emscripten/release/deps -type f ! -name "*.asm.js" -name "*.js" | xargs -I {} cat {} >> $@
	@echo "emscripten js files updated"

target/wasm32-unknown-emscripten/release/deps:
	@cargo build --target=wasm32-unknown-emscripten --release
	@echo "wasm built using emscripten"

package: wasm
	npm install
	@electron-forge make
	@ls out/make

clean:
	rm -rf target
	rm -rf out
	rm src/flowui.js src/flowui.wasm

init:
	rustup target add wasm32-unknown-emscripten
	# curl https://s3.amazonaws.com/mozilla-games/emscripten/releases/emsdk-portable.tar.gz | tar -xv -C ~/
    # cd ~/emsdk-portable
    # ./emsdk update
    # ./emsdk install sdk-incoming-64bit
    # ./emsdk activate sdk-incoming-64bit
    # emcc -v