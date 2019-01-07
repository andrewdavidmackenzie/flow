RUSTUP := $(shell command -v rustup 2> /dev/null)
TARGETDIR = ../ui-target
OUTDIR = out

all: app web

run: app
	electron-forge start

app: node_modules src/electronflow.html src/electron-main.js

web: src/index.html
	@python3 -m http.server

src/electronflow.html: src/electronflowui.js src/%.wasm src/renderer.js

src/index.html: src/flowui.js src/%.wasm

node_modules:
	@npm install

src/%.wasm: $(TARGETDIR)/wasm32-unknown-unknown/release/deps
	@cp $(TARGETDIR)/wasm32-unknown-unknown/release/deps/*.wasm src/
	@echo "Copied wasm files in '$^/' to 'src/'"

src/flowui.js: $(TARGETDIR)/wasm32-unknown-unknown/release/deps
	@find $(TARGETDIR)/wasm32-unknown-unknown/release/deps -type f ! -name "*.asm.js" -name "*.js" | xargs -I {} cp {} $@
	@echo "Copied compiled js from '$(TARGETDIR)' into $@"

src/electronflowui.js: src/electron-prefix.js src/flowui.js
	@cat $^ > $@

$(TARGETDIR)/wasm32-unknown-unknown/release/deps: rustup-target ../flowclib src
# Compile output into a folder not under here to avoid it being packaged by electron-forge
# since I cannot get the ignore[] part of config in package.json to ignore it when using electron-forge make
	@CARGO_TARGET_DIR=$(TARGETDIR) cargo build --target=wasm32-unknown-unknown --release
	@echo "Compiled rust source in 'src/' to wasm in '$@/'"

package: app package.json
	@electron-forge make
	@echo "Package created in '$(OUTDIR)/make/' folder"
	@ls $(OUTDIR)/make/*.dmg

clean:
	rm -rf $(TARGETDIR)
	rm -rf $(OUTDIR)
	rm -f src/electronflowui.js src/*.wasm

check: rustup-target

rustup-target:
ifdef RUSTUP
	@rustup target add wasm32-unknown-unknown
else
	$(error "rustup must be installed to run 'rustup target add wasm32-unknown-unknown' to add wasm target for build. Please install or update Cargo and rustup.")
endif