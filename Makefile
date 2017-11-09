EMCC := $(shell command -v emcc -v 2> /dev/null)
RUSTUP := $(shell command -v rustup 2> /dev/null)

all: cli app

app: wasm
	electron-forge start

cli:
	@cargo build --bin flow

wasm: src/flowui.wasm src/flowui.js node_modules

src/flowui.wasm: target/wasm32-unknown-emscripten/release/deps
	@find target/wasm32-unknown-emscripten/release/deps -type f -name "*.wasm" | xargs -I {} cp {} $@
	@echo "emscripten wasm files updated"

src/flowui.js: target/wasm32-unknown-emscripten/release/deps
	@cp src/electron-prefix.js $@
	@find target/wasm32-unknown-emscripten/release/deps -type f ! -name "*.asm.js" -name "*.js" | xargs -I {} cat {} >> $@
	@echo "emscripten js files updated"

target/wasm32-unknown-emscripten/release/deps: emscipten
	@cargo build --bin ui --target=wasm32-unknown-emscripten --release
	@echo "wasm built using emscripten"

node_modules:
	@npm install

package: wasm
	@electron-forge make
	@ls out/make

test:
	@cargo test
	@cd lib
	@cargo test

clean:
	rm -rf target
	rm -rf out
	rm src/flowui.js src/flowui.wasm

emscipten: rustup-target emcc

rustup-target:
ifndef RUSTUP
    $(error "rustup must be installed to add wasm target for build.")
else
	@rustup target add wasm32-unknown-emscripten
endif

emcc:
ifndef EMCC
	$(MAKE) install-emcc
    $(error "emcc must be installed to compile wasm. Try 'make install-emcc'")
endif

install-emcc:
	@echo "Install emcc using something like this:"
	@echo "	curl https://s3.amazonaws.com/mozilla-games/emscripten/releases/emsdk-portable.tar.gz | tar -xv -C ~/"
	@echo "	cd ~/emsdk-portable"
	@echo "	./emsdk update"
	@echo "	./emsdk install sdk-incoming-64bit"
	@echo "	./emsdk activate sdk-incoming-64bit"
	@echo "Then check that 'emcc -v' works"
