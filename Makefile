DOT := $(shell command -v dot 2> /dev/null)
KCOV := $(shell command -v kcov 2> /dev/null)
STIME = @mkdir -p target;date '+%s' > target/.$@time ; echo \\n------- Target \'$@\' starting
ETIME = @read st < target/.$@time ; st=$$((`date '+%s'`-$$st)) ; echo ------- Target \'$@\' done in $$st seconds
FLOWSTDLIB_FILES = $(shell find flowstdlib -type f | grep -v manifest.json)
UNAME := $(shell uname)

all:
	$(STIME)
	@PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:/usr/local/opt/lib/pkgconfig:/usr/local/Cellar/glib/2.62.3/lib/pkgconfig:/usr/lib64/pkgconfig"
	@$(MAKE) workspace test docs
	$(ETIME)

online := false

ifeq ($(online),true)
features := --features "online_tests"
else
features :=
endif

########## Configure Dependencies ############
config: common-config
	$(STIME)
	@echo "Detected OS=$(UNAME)"
ifeq ($(UNAME), Linux)
	@$(MAKE) config-linux
endif
ifeq ($(UNAME), Darwin)
	@$(MAKE) config-darwin
endif
	$(ETIME)

common-config:
	export PATH="$PATH:~/.cargo/bin"
	rustup target add wasm32-unknown-unknown
	# cargo install wasm-gc || true
	# install mdbook for generating guides
	cargo install mdbook --root . --git https://github.com/andrewdavidmackenzie/mdbook || true
	#cargo install mdbook-linkcheck --root . || true

config-darwin:
	$(STIME)
	brew install gtk glib cairo cmake
	$(ETIME)

config-linux:
	$(STIME)
	sudo apt-get -y install libcurl4-openssl-dev libelf-dev libdw-dev libssl-dev binutils-dev
	$(ETIME)

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

################### Coverage ####################
.PHONY: coverage
coverage: build-kcov measure #upload_coverage

COVERAGE_PREFIXES := "flow_impl-*" "runtime-*" "provider-*" "flow_impl_derive-*" "flowc-*" "flowstdlib-*" "flowr-*" "flowrlib-*"
# flowc_*-* and flowr_*-*

upload_coverage: $(COVERAGE_PREFIXES)
	@echo "Uploading coverage to https://codecov.io....."
	@curl -s https://codecov.io/bash | bash

measure: $(COVERAGE_PREFIXES)

$(COVERAGE_PREFIXES):
	@coverage.sh $@

build-kcov:
ifeq ($(KCOV),)
	@echo "'kcov' is not installed. Building and installing it"
	@printf "Building 'kcov' from source and installing it"
	@wget https://github.com/SimonKagstrom/kcov/archive/master.tar.gz
	@rm -rf kcov-master
	@tar xzf master.tar.gz
ifeq ($(UNAME), Linux)
	@cd kcov-master && rm -rf build && mkdir build && cd build && cmake .. && make && sudo make install
endif
ifeq ($(UNAME), Darwin)
	@cd kcov-master && rm -rf build && mkdir build && cd build && cmake -G Xcode .. &&  xcodebuild -configuration Release§
	@sudo mv kcov-master/build/src/Debug/kcov /usr/local/bin/kcov
endif
	@rm -rf kcov-master
	@rm -f master.tar.gz*
else
	@echo "'kcov' found, skipping build of it"
endif

#################### FLOW LIBRARIES ####################
flowstdlib: flowstdlib/manifest.json

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
#	@cd samples; $(MAKE) clean
	@$(MAKE) $(sample_flows)
	$(ETIME)

samples/%: samples/%/test.err
	$(MAKE) $(@D)/test.output

samples/%/test.output: samples/%/test.input samples/%/test.arguments
	@printf "\tSample '$(@D)'"
	@RUST_BACKTRACE=1 cat $< | cargo run --quiet -p flowc -- -g -d $(@D) -- `cat $(@D)/test.arguments` 2> $(@D)/test.err > $@
	@diff $@ $(@D)/expected.output || (ret=$$?; cp $@ $(@D)/failed.output && rm -f $@ && exit $$ret)
	@if [ -s $(@D)/test.err ]; then (printf " has error output in $(@D)/test.err\n"; exit -1); else printf " has no errors\n"; fi;
	@rm $@ #remove test.output after successful diff so that dependency will cause it to run again next time
# leave test.err for inspection in case of failure

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