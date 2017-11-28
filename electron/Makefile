# TODO rustc -O --target wasm32-unknown-unknown main.rs
# TODO this should work very soon and we can dump emscripten
# as rustc nightly can have a wasm target directly....
# TODO for now we have to do this...
# wasm-gc main.wasm main.gc.wasm - to add garbage collection to it!


EMCC := $(shell command -v emcc -v 2> /dev/null)
RUSTUP := $(shell command -v rustup 2> /dev/null)
TARGETDIR = ../ui-target
OUTDIR = out

all: app web

run: app
	electron-forge start

app: node_modules src/electronflow.html src/electron-main.js

web: src/index.html
	@echo open browser to http://localhost:8000/src/
	@python2.7 -m SimpleHTTPServer

src/electronflow.html: src/electronflowui.js src/%.wasm src/renderer.js

src/index.html: src/flowui.js src/%.wasm

node_modules:
	@npm install

src/%.wasm: $(TARGETDIR)/wasm32-unknown-emscripten/release/deps
	@cp $(TARGETDIR)/wasm32-unknown-emscripten/release/deps/*.wasm src/
	@echo "Copied wasm files in '$^/' to 'src/'"

src/flowui.js: $(TARGETDIR)/wasm32-unknown-emscripten/release/deps
	@find $(TARGETDIR)/wasm32-unknown-emscripten/release/deps -type f ! -name "*.asm.js" -name "*.js" | xargs -I {} cp {} $@
	@echo "Copied compiled js from '$(TARGETDIR)' into $@"

src/electronflowui.js: src/electron-prefix.js src/flowui.js
	@cat $^ > $@

$(TARGETDIR)/wasm32-unknown-emscripten/release/deps: ../lib rustup-target emcc src
# Compile output into a folder not under here to avoid it being packaged by electron-forge
# since I cannot get the ignore[] part of config in package.json to ignore it when using electron-forge make
	@CARGO_TARGET_DIR=$(TARGETDIR) cargo build --bin ui --target=wasm32-unknown-emscripten --release
	@echo "Compiled rust source in 'src/' to wasm in '$@/' using emscripten"

package: app package.json
	@electron-forge make
	@echo "Package created in '$(OUTDIR)/make/' folder"
	@ls $(OUTDIR)/make/*.dmg

clean:
	rm -rf $(TARGETDIR)
	rm -rf $(OUTDIR)
	rm -f src/electronflowui.js src/*.wasm

check: rustup-target emcc

rustup-target:
ifdef RUSTUP
	@rustup target add wasm32-unknown-emscripten
else
	$(error "rustup must be installed to run 'rustup target add wasm32-unknown-emscripten' to add wasm target for build. Please install or update Cargo and rustup.")
endif

emcc:
ifndef EMCC
	$(error "'emcc' must be installed to be able to compile rust to wasm. Use 'make install-emcc' for instructions")
endif

install-emcc:
	@echo "Install emcc using something like this:"
	@echo "	curl https://s3.amazonaws.com/mozilla-games/emscripten/releases/emsdk-portable.tar.gz | tar -xv -C ~/"
	@echo "	cd ~/emsdk-portable"
	@echo "	./emsdk update"
	@echo "	./emsdk install sdk-incoming-64bit"
	@echo "	./emsdk activate sdk-incoming-64bit"
	@echo "Then check that 'emcc -v' works"