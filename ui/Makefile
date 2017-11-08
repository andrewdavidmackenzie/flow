run: wasm
	electron-forge start	

wasm: build-wasm copy-wasm
	@echo "wasm built and copied"

# Build the rust project, generating wasm and js files in ./target/
build-wasm:
	@cargo build --target=wasm32-unknown-emscripten --release

# Copy generated JS and wasm files from target directory to src directory
copy-wasm: src/flowui.wasm src/flowui.js

src/flowui.wasm:
	@find target/wasm32-unknown-emscripten/release/deps -type f -name "*.wasm" | xargs -I {} cp {} $@

src/flowui.js:
	@cp src/electron-prefix.js $@
	@find target/wasm32-unknown-emscripten/release/deps -type f ! -name "*.asm.js" -name "*.js" | xargs -I {} cat {} >> $@

package:
	npm install
	electron-forge make

clean:
	rm -rf target
	rm src/flowui.js src/flowui.wasm

init:
	rustup target add wasm32-unknown-emscripten
	# curl https://s3.amazonaws.com/mozilla-games/emscripten/releases/emsdk-portable.tar.gz | tar -xv -C ~/
    # cd ~/emsdk-portable
    # ./emsdk update
    # ./emsdk install sdk-incoming-64bit
    # ./emsdk activate sdk-incoming-64bit
    # emcc -v