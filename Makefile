RUSTUP := $(shell command -v rustup 2> /dev/null)
DOT := $(shell command -v dot 2> /dev/null)

all: build test doc
	@echo ""
	@echo "**************************************"
	@echo "************* Done all: **************"
	@echo "**************************************"

online := false

ifeq ($(online),true)
features := --features "online_tests"
else
features :=
endif

########## Configure Dependencies ############
config:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-bindgen-cli || true
	cargo install wasm-gc || true
	# install mdbook for generating guides
	cargo install mdbook --git https://github.com/andrewdavidmackenzie/mdbook || true
	cargo install mdbook-linkcheck || true
	# install wasm-pack
	curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh -s -- -f
	# Install chromedriver.
	#curl --retry 5 -LO https://chromedriver.storage.googleapis.com/2.41/chromedriver_linux64.zip
	#unzip chromedriver_linux64.zip

config-linux:
	brew install fakeroot

################### Doc ####################
doc: build-guide clean-guide code-docs

code-docs:
	@echo "------- Building code docs -------------"
	@cargo doc --no-deps

build-guide:
	@echo "------- Building guide mdbook from Markdown -------------"
	@../mdbook/target/debug/mdbook build

clean-guide:
	@rm -rf target/html/nodeprovider/target
	@rm -rf target/html/ide/target
	@rm -rf target/html/flowrlib/target
	@rm -rf target/html/flowstdlib/target
	@rm -rf target/html/samples/mandlebrot/project/target
	@rm -rf target/html/ide-native/target
	@rm -rf target/html/flowclib/target
	@find target/html -name node_modules | xargs rm -rf {}
	@find target/html -name .idea | xargs rm -rf {}
	@find target/html -name .git | xargs rm -rf {}
	@find target/html -name assets | xargs rm -rf {}
	@find target/html -name \*.rs | xargs rm -rf {}
	@find target/html -name pkg | xargs rm -rf {}
	@find target/html -name \*.dump | xargs rm -rf {}
	@find target/html -name \*.dot | xargs rm -rf {}

.PHONY: deploy
deploy: docs/guide
	@echo "====> deploying guide to github"
	git worktree add /tmp/guide gh-pages
	rm -rf /tmp/guide/*
	cp -rp target/guide/html/* /tmp/guide/
	cd /tmp/guide && \
		git add -A && \
		git commit -m "deployed on $(shell date) by ${USER}" && \
		git push origin gh-pages

#################### Build ####################
build: workspace ide_build ide_native_build
	@echo "------- Done 'build:' -------------"

flowcompiler:
	@echo ""
	@echo "------- Starting build of 'flowc'                  -------------"
	@cargo build -p flowc
	@echo "------- Done     build of 'flowc'                  -------------"

workspace: flowstdlib/manifest.json
	@echo ""
	@echo "------- Starting build of 'flow' workspace project -------------"
	@cargo build --all
	@echo "------- Done     build of 'flow' workspace project -------------"

flowr:
	@cargo build -p flowr

ide_build:
	@cd ide && make build

ide_native_build:
	@cd ide-native && make build

#################### Tests ####################
test: test-workspace test-ide samples book-test
# TODO add online-samples
	@echo ""
	@echo "------- Done    test: -------------"

test-workspace:
	@echo ""
	@echo "------- Starting test of workspace project -------------"
	@cargo test $(features) --all
	@echo "------- Done     test of workspace project -------------"

test-ide:
	@cd ide && make test

book-test:
	@mdbook test

#################### LIBRARIES ####################
flowstdlib/manifest.json: flowcompiler
	@echo ""
	@echo "------- Starting build of 'flowstdlib' -------------"
	@cargo run -p flowc -- -v info -l flowstdlib
	@echo "------- Done     build of 'flowstdlib' -------------"

#################### Raspberry Pi ####################
pi:
	@echo "Building flowc for pi in $(PWD)"
# https://hub.docker.com/r/dlecan/rust-crosscompiler-arm
	docker run -it --rm -v $(PWD):/source -v ~/.cargo/git:/root/.cargo/git -v ~/.cargo/registry:/root/.cargo/registry dlecan/rust-crosscompiler-arm:stable
# In case of permissions problems for cargo cache on local machine:
# sudo chown -R `stat -c %u:%g $HOME` $(pwd) ~/.cargo

copy:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/release/flowc andrew@zero-w:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/release/flowr andrew@zero-w:

#################### SAMPLES ####################
# Find all sub-directories under 'samples' and create a list of paths like 'sample/{directory}/test.output' to use for
# make paths - to compile all samples found in there. Avoid files using the filter.
sample_flows := $(patsubst samples/%,samples/%test.output,$(filter %/, $(wildcard samples/*/)))

samples: workspace flowr clean-samples $(sample_flows)  # This target must be below sample-flows in the Makefile
	@echo ""
	@echo "All local samples executed and output as expected"
	@echo "------- Finished 'samples:' ----"

samples/%/test.output: samples/%/test.input samples/%/test.arguments
	@echo "\n------- Compiling and Running '$(@D)' ----"
# remove error messages with file path from output messages to make local output match travis output
	@cat $< | cargo run --quiet --bin flowc -- -g -d $(@D) -- `cat $(@D)/test.arguments` | grep -v "Running" | grep -v "Finished dev" 2> $(@D)/test.err > $@; true
	@diff $@ $(@D)/expected.output || (ret=$$?; cp $@ $(@D)/failed.output && rm -f $@ && exit $$ret)
	@echo "Sample output matches expected.output"
	@rm $@ #remove test.output after successful diff so that dependency will cause it to run again next time

################# ONLINE SAMPLES ################
online-samples: test-hello-simple-online

test-hello-simple-online: ./target/debug/flowc
	@echo ""
	@echo "------- Testing hello-world-simple-online ----"
	@echo "Hello" | cargo run --bin flowc -- https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml

################# Packaging ################
publish:
	cargo publish --manifest-path=flow_impl/Cargo.toml || true
	cargo publish --manifest-path=flow_impl_derive/Cargo.toml || true
	cargo publish --manifest-path=flowrlib/Cargo.toml || true
	cargo publish --manifest-path=provider/Cargo.toml || true
	cargo publish --manifest-path=flowclib/Cargo.toml || true
	cargo publish --manifest-path=flowc/Cargo.toml || true
	cargo publish --manifest-path=flowr/Cargo.toml || true

################# Clean ################
clean: clean-flowstdlib clean-samples clean-dumps clean-guide
	@cargo clean
	@cd ide && make clean
	@cd ide-native && make clean

clean-samples:
	@cd samples; make clean

clean-flowstdlib:
	@find flowstdlib -name \*.wasm -type f -exec rm -rf {} + ; true
	@rm -f flowstdlib/manifest.json

clean-dumps:
	@find . -name \*.dump -type f -exec rm -rf {} + ; true
	@find . -name \*.dot -type f -exec rm -rf {} + ; true
	@find . -name \*.dot.png -type f -exec rm -rf {} + ; true
	@echo "All .dump, .dot and .dot.png files removed"

clean-guide:
	@rm -rf guide/book

################# Dot Graphs ################
dot-graphs:
ifeq ($(DOT),)
	@echo "'dot' not available, skipping 'dot-graphs'. Install 'graphviz' to use."
else
	@find . -name \*.dot -type f -exec dot -Tpng -O {} \;
	@echo "Generated .png files for all dot graphs found"
endif