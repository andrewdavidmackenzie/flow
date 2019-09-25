RUSTUP := $(shell command -v rustup 2> /dev/null)
DOT := $(shell command -v dot 2> /dev/null)
STIME = @date '+%s' > target/.$@time ; echo \\n------- Target \'$@\' starting
ETIME = @read st < target/.$@time ; st=$$((`date '+%s'`-$$st)) ; echo ------- Target \'$@\' done in $$st seconds

all:
	$(STIME)
	@$(MAKE) travis ide_build ide_native_build test-ide
	$(ETIME)

travis:
	$(STIME)
	@$(MAKE) workspace test-workspace samples book-test doc
	$(ETIME)

online := false

ifeq ($(online),true)
features := --features "online_tests"
else
features :=
endif

########## Configure Dependencies ############
config:
	$(STIME)
	rustup target add wasm32-unknown-unknown
	cargo install wasm-bindgen-cli || true
	# cargo install wasm-gc || true
	# install mdbook for generating guides
	cargo uninstall mdbook
	cargo install mdbook --git https://github.com/andrewdavidmackenzie/mdbook || true
	cargo install mdbook-linkcheck || true
	# install wasm-pack
	curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh -s -- -f
	# Install chromedriver.
	#curl --retry 5 -LO https://chromedriver.storage.googleapis.com/2.41/chromedriver_linux64.zip
	#unzip chromedriver_linux64.zip
	$(ETIME)

config-linux:
	$(STIME)
	brew install fakeroot
	$(ETIME)

################### Doc ####################
doc: build-guide trim-guide code-docs

build-guide:
	$(STIME)
	@RUST_LOG=info mdbook build
	$(ETIME)

trim-guide:
	$(STIME)
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
	@find target/html -name \*.wasm | xargs rm -rf {}
	@find target/html -name \*.lock  | xargs rm -rf {}
	$(ETIME)

code-docs:
	$(STIME)
	@cargo doc --no-deps
	$(ETIME)

.PHONY: deploy
deploy: docs/guide
	$(STIME)
	@echo "====> deploying guide to github"
	git worktree add /tmp/guide gh-pages
	rm -rf /tmp/guide/*
	cp -rp target/guide/html/* /tmp/guide/
	cd /tmp/guide && \
		git add -A && \
		git commit -m "deployed on $(shell date) by ${USER}" && \
		git push origin gh-pages
	$(ETIME)

#################### Build ####################
build:
	$(STIME)
	@$(MAKE) workspace ide_build ide_native_build
	$(ETIME)

flowcompiler:
	$(STIME)
	@cargo build -p flowc
	$(ETIME)

workspace: flowstdlib/manifest.json
	$(STIME)
	@cargo build --all
	$(ETIME)

flowr:
	$(STIME)
	@cargo build -p flowr
	$(ETIME)

ide_build:
	$(STIME)
	@cd ide && make build
	$(ETIME)

ide_native_build:
	$(STIME)
	@cd ide-native && make build
	$(ETIME)

#################### Tests ####################
test:
	$(STIME)
	@$(MAKE) test-workspace test-ide samples book-test
	$(ETIME)

# TODO add online-samples

test-workspace:
	$(STIME)
	@cargo test $(features) --all
	$(ETIME)

test-ide:
	$(STIME)
	@cd ide && make test
	$(ETIME)

book-test:
	$(STIME)
	@RUST_LOG=info mdbook test
	$(ETIME)

#################### LIBRARIES ####################
flowstdlib/manifest.json: flowcompiler
	@cargo run -p flowc -- -v info -l flowstdlib

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

# This target must be below sample-flows in the Makefile
samples:
	$(STIME)
	$(MAKE) workspace flowr clean-samples $(sample_flows)
	$(ETIME)

samples/%/test.output: samples/%/test.input samples/%/test.arguments
# remove error messages with file path from output messages to make local output match travis output
	@cat $< | cargo run --quiet --bin flowc -- -g -d $(@D) -- `cat $(@D)/test.arguments` | grep -v "Running" | grep -v "Finished dev" 2> $(@D)/test.err > $@; true
	@diff $@ $(@D)/expected.output || (ret=$$?; cp $@ $(@D)/failed.output && rm -f $@ && exit $$ret)
	@echo "Sample output matches expected.output"
	@rm $@ #remove test.output after successful diff so that dependency will cause it to run again next time

################# ONLINE SAMPLES ################
online-samples: test-hello-simple-online

test-hello-simple-online: flowcompiler
	$(STIME)
	@echo "Hello" | cargo run --bin flowc -- https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml
	$(ETIME)

################# Packaging ################
publish:
	$(STIME)
	cargo publish --manifest-path=flow_impl/Cargo.toml || true
	cargo publish --manifest-path=flow_impl_derive/Cargo.toml || true
	cargo publish --manifest-path=flowrlib/Cargo.toml || true
	cargo publish --manifest-path=provider/Cargo.toml || true
	cargo publish --manifest-path=flowclib/Cargo.toml || true
	cargo publish --manifest-path=flowc/Cargo.toml || true
	cargo publish --manifest-path=flowr/Cargo.toml || true
	$(ETIME)

################# Clean ################
clean: clean-flowstdlib clean-samples clean-dumps clean-guide
	$(STIME)
	@cargo clean
	@cd ide && make clean
	@cd ide-native && make clean
	$(ETIME)

clean-samples:
	$(STIME)
	@cd samples; make clean
	$(ETIME)

clean-flowstdlib:
	$(STIME)
	@find flowstdlib -name \*.wasm -type f -exec rm -rf {} + ; true
	@rm -f flowstdlib/manifest.json
	$(ETIME)

clean-dumps:
	$(STIME)
	@find . -name \*.dump -type f -exec rm -rf {} + ; true
	@find . -name \*.dot -type f -exec rm -rf {} + ; true
	@find . -name \*.dot.png -type f -exec rm -rf {} + ; true
	@echo "All .dump, .dot and .dot.png files removed"
	$(ETIME)

clean-guide:
	$(STIME)
	@rm -rf guide/book
	$(ETIME)

################# Dot Graphs ################
dot-graphs:
ifeq ($(DOT),)
	@echo "'dot' not available, skipping 'dot-graphs'. Install 'graphviz' to use."
else
	@find . -name \*.dot -type f -exec dot -Tpng -O {} \;
	@echo "Generated .png files for all dot graphs found"
endif