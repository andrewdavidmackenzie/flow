DOT := $(shell command -v dot 2> /dev/null)
KCOV := $(shell command -v kcov 2> /dev/null)
STIME = @mkdir -p target;date '+%s' > target/.$@time ; echo \\n------- Target \'$@\' starting
ETIME = @read st < target/.$@time ; st=$$((`date '+%s'`-$$st)) ; echo ------- Target \'$@\' done in $$st seconds
FLOWSTDLIB_FILES = $(shell find flowstdlib -type f | grep -v manifest.json)
UNAME := $(shell uname)

all:
	$(STIME)
	#@PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:/usr/local/opt/lib/pkgconfig:/usr/local/Cellar/glib/2.62.2/lib/pkgconfig:/usr/lib64/pkgconfig"
	$(MAKE) workspace test-workspace samples book-test docs
	$(ETIME)

online := false

ifeq ($(online),true)
features := --features "online_tests"
else
features :=
endif

########## Configure Dependencies ############
config: travis-config
ifeq ($(UNAME), Linux)
	@$(MAKE) config-linux
endif
ifeq ($(UNAME), Darwin)
	@$(MAKE) config-darwin
endif

travis-config:
	$(STIME)
	@echo "Detected OS=$(UNAME)"
	rustup target add wasm32-unknown-unknown
	# cargo install wasm-gc || true
	# install mdbook for generating guides
	cargo install mdbook --root . --git https://github.com/andrewdavidmackenzie/mdbook || true
	#cargo install mdbook-linkcheck --root . || true
	$(ETIME)

config-darwin:
	$(STIME)
	brew install gtk+3
	$(ETIME)

config-linux:
	$(STIME)
	$(ETIME)

################### Coverage ####################
kcov:
	wget https://github.com/SimonKagstrom/kcov/archive/master.tar.gz
	tar xzf master.tar.gz
	cd kcov-master
	mkdir build
	cd build
#Mac	cmake -G Xcode ..
	cmake ..
#Mac	xcodebuild -configuration Release
#Mac mv src/Release/kcov ../../bin
	make
	sudo make install
	cd ../..
	rm -rf kcov-master

################### Doc ####################
.PHONY: docs
docs:
	$(STIME)
	@$(MAKE) build-guide trim-guide code-docs
	$(ETIME)

build-guide:
	@RUST_LOG=info time ./bin/mdbook build

trim-guide:
	$(STIME)
	@find target/html -name target -type d | xargs rm -rf {}
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
	@cargo doc --all --quiet --no-deps --target-dir=target/html/code
	$(ETIME)

.PHONY: deploy
deploy: build_guide
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
workspace:
	$(STIME)
	@cargo build $(features) --all
	$(ETIME)

flowrunner:
	$(STIME)
	@cargo build -p flowr
	$(ETIME)

#################### Tests ####################
test:
	$(STIME)
	@$(MAKE) test-workspace samples book-test
	$(ETIME)

test-workspace:
	$(STIME)
	@cargo test $(features) --all
	$(ETIME)

book-test:
	$(STIME)
	./bin/mdbook test
	$(ETIME)

.PHONY: coverage
coverage:
ifeq ($(DOT),)
	@echo "\t'kcov' not available. Building and installing it"
	$(MAKE) kcov
else
	for file in target/debug/*.d; do mkdir -p "target/cov/$(basename $file)"; kcov target/cov --exclude-pattern=/.cargo,/usr/lib --verify "target/cov/$(basename $file)" "$file"; done
endif

upload-coverage:
	bash <(curl -s https://codecov.io/bash)

#################### LIBRARIES ####################
flowstdlib/manifest.json: $(FLOWSTDLIB_FILES)
	@mkdir -p target;date '+%s' > target/.flowstdlibtime ; echo \\n------- Target \'$@\' starting
	@cargo run -p flowc -- -v info -l flowstdlib
	@read st < target/.flowstdlibtime ; st=$$((`date '+%s'`-$$st)) ; echo ------- Target \'$@\' done in $$st seconds

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
samples: flowrunner flowstdlib/manifest.json
	$(STIME)
	@cd samples; $(MAKE) clean
	@$(MAKE) $(sample_flows)
	$(ETIME)

samples/%/test.output: samples/%/test.input samples/%/test.arguments
# remove error messages with file path from output messages to make local output match travis output
	@cat $< | cargo run --quiet --bin flowc -- -g -d $(@D) -- `cat $(@D)/test.arguments` | grep -v "Running" | grep -v "Finished dev" 2> $(@D)/test.err > $@; true
	@diff $@ $(@D)/expected.output || (ret=$$?; cp $@ $(@D)/failed.output && rm -f $@ && exit $$ret)
	@echo "\tSample '$(@D)' output matches expected.output"
	@rm $@ #remove test.output after successful diff so that dependency will cause it to run again next time

################# ONLINE SAMPLES ################
online-samples:
	$(STIME)
	@echo "Hello" | cargo run --p flowc -- https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml
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
clean:
	$(STIME)
	@$(MAKE) clean-dumps clean-guide
	@cd flowstdlib; $(MAKE) clean
	@cd samples; $(MAKE) clean
	@cargo clean
	$(STIME)

clean-dumps:
	$(STIME)
	@find . -name \*.dump -type f -exec rm -rf {} + ; true
	@find . -name \*.dot -type f -exec rm -rf {} + ; true
	@find . -name \*.dot.png -type f -exec rm -rf {} + ; true
	@echo "\tAll .dump, .dot and .dot.png files removed"
	$(ETIME)

clean-guide:
	$(STIME)
	@rm -rf target/html
	$(ETIME)

################# Dot Graphs ################
dot-graphs:
ifeq ($(DOT),)
	@echo "\t'dot' not available, skipping 'dot-graphs'. Install 'graphviz' to use."
else
	@find . -name \*.dot -type f -exec dot -Tpng -O {} \;
	@echo "\tGenerated .png files for all dot graphs found"
endif