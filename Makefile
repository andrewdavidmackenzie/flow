DOT := $(shell command -v dot 2> /dev/null)
KCOV := $(shell command -v kcov 2> /dev/null)
APTGET := $(shell command -v apt-get 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
STIME = @mkdir -p target;date '+%s' > target/.$@time ; echo "<------ Target '$@' starting"
ETIME = @read st < target/.$@time ; st=$$((`date '+%s'`-$$st)) ; echo "------> Target '$@' done in $$st seconds"
SOURCES = $(shell find . -type f -name \*.rs)
MARKDOWN = $(shell find . -type f -name \*.md)
DOTS = $(shell find . -type f -name \*.dot)
SVGS = $(patsubst %.dot,%.dot.svg,$(DOTS))
FLOWSTDLIB_SOURCES = $(shell find flowstdlib -type f -name \*.rs)
FLOWSTDLIB_TOMLS = $(shell find flowstdlib -type f -name \*.toml)
FLOWSTDLIB_MARKDOWN = $(shell find flowstdlib -type f -name \*.md)
UNAME := $(shell uname)
ONLINE := $(shell ping -q -c 1 -W 1 8.8.8.8 2> /dev/null)
export FLOW_ROOT := $(dir $(realpath $(firstword $(MAKEFILE_LIST))))
export SHELL := /bin/bash

.PHONY: all
all: clippy build test samples docs

########## Configure Dependencies ############
.PHONY: config
config: common-config
	$(STIME)
	@echo "Detected $(UNAME)"
ifeq ($(UNAME), Linux)
	@$(MAKE) config-linux
endif
ifeq ($(UNAME), Darwin)
	@$(MAKE) config-darwin
endif
	$(ETIME)

.PHONY: clippy-config
clippy-config:
	$(STIME)
	@echo "	Installing clippy command using rustup"
	@export PATH="$$PATH:~/.cargo/bin"
	@rustup --quiet component add clippy
	$(ETIME)

.PHONY: wasm-config
wasm-config:
	$(STIME)
	@echo "	Installing wasm32 target using rustup"
	@rustup --quiet target add wasm32-unknown-unknown
	$(ETIME)

.PHONY: book-config
book-config:
	$(STIME)
	@echo "	Installing mdbook and mdbook-linkcheck using cargo"
	@cargo install mdbook
	@cargo install mdbook-linkcheck
	$(ETIME)

.PHONY: common-config
common-config: no-book-config book-config

.PHONY: no-book-config
no-book-config: clippy-config wasm-config

.PHONY: config-darwin
config-darwin:
	$(STIME)
	@echo "	Installing macos specific dependencies using brew"
	@brew install gtk+3 glib cairo atk cmake graphviz zmq
	$(ETIME)

.PHONY: config-linux
config-linux:
	$(STIME)
ifneq ($(YUM),)
	@echo "	Installing linux specific dependencies using $(YUM)"
	@sudo yum --color=auto --quiet install curl-devel elfutils-libelf-devel elfutils-devel openssl-devel binutils-devel || true
	@sudo yum --color=auto --quiet install graphviz gtk3-devel zeromq zeromq-devel || true
else ifneq ($(APTGET),)
	@echo "	Installing linux specific dependencies using $(APTGET)"
	@sudo apt-get -y install libcurl4-openssl-dev libelf-dev libdw-dev libssl-dev binutils-dev || true
	@sudo apt-get -y install graphviz libgtk-3-dev libzmq3-dev || true
else
	@echo "	Neither apt-get nor yum detected for installing linux specific dependencies"
endif
	$(ETIME)

################### Doc ####################
.PHONY: docs
docs: book code-docs trim-docs

.PHONY: book
book: target/html/index.html

target/html/index.html: $(MARKDOWN) $(SVGS) dot-graphs
	@RUST_LOG=info time mdbook build

%.dot.svg: %.dot
ifeq ($(DOT),)
	@echo "        Install 'graphviz' to be able to generate dot graphs for flows."
else
	@dot -Tsvg -O $<
	@echo "        Generated $@ SVG file from $< dot file"
endif

dot-graphs:
	$(STIME)
ifeq ($(DOT),)
	@echo "        'dot' not available, skipping 'dot-graphs'. Install 'graphviz' to use."
else
	@echo "        Generated .svg files for all dot graphs found"
	@find . -name \*.dot -type f -exec dot -Tsvg -O {} \;
endif
	$(ETIME)

.PHONY: trim-docs
trim-docs:
	$(STIME)
	@find target/html -name target -type d | xargs rm -rf {}
	@find target/html -name .idea | xargs rm -rf {}
	@find target/html -name \*.iml | xargs rm -rf {}
	@find target/html -name .git | xargs rm -rf {}
	@find target/html -name .sh | xargs rm -rf {}
	@find target/html -name assets | xargs rm -rf {}
	@find target/html -name Cargo.toml | xargs rm -rf {}
	@find target/html -name manifest.json | xargs rm -rf {}
	@find target/html -name test.err | xargs rm -rf {}
	@find target/html -name \*.rs | xargs rm -rf {}
	@find target/html -name pkg | xargs rm -rf {}
	@find target/html -name \*.dump | xargs rm -rf {}
	@find target/html -name \*.dot | xargs rm -rf {}
	@find target/html -name \*.wasm | xargs rm -rf {}
	@find target/html -name \*.lock  | xargs rm -rf {}
	@cd target/html && rm -f Makefile .crates.toml .DS_Store .gitignore .mdbookignore .travis.yml
	@cd target/html && rm -rf bin
	@rm -rf target/html/flowc/tests/test-flows
	@rm -rf target/html/flowc/tests/test-libs
	@rm -rf target/html/code/debug
	@find target/html -depth -type d -empty -delete
	$(ETIME)

.PHONY: code-docs
code-docs: $(SOURCES)
	$(STIME)
	@cargo doc --workspace --quiet --no-deps --target-dir=target/html/code
	$(ETIME)

.PHONY: pages
pages: docs deploy-pages

.PHONY: deploy-pages
deploy-pages:
	$(STIME)
	@echo "====> deploying guide to github"
	git worktree prune
	@rm -rf /tmp/guide
	git worktree add /tmp/guide gh-pages
	rm -rf /tmp/guide/*
	cp -rp target/html/* /tmp/guide/
	cd /tmp/guide && \
		git add -A && \
		git commit -m "deployed on $(shell date) by ${USER}" && \
		git push --force origin gh-pages
	$(ETIME)

#################### Build ####################
.PHONY: build
build: $(SOURCES) flowstdlib
	$(STIME)
	@PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:/usr/local/opt/lib/pkgconfig:/usr/local/Cellar/glib/2.62.3/lib/pkgconfig:/usr/lib64/pkgconfig" cargo build
	$(ETIME)

.PHONY: clippy
clippy: $(SOURCES)
	$(STIME)
	@cargo clippy -- -D warnings
	$(ETIME)

#################### Tests ####################
.PHONY: test
test: $(SOURCES)
	$(STIME)
	@set -o pipefail && cargo test --workspace --exclude flow_impl_derive --exclude flowide -- --test-threads 1 2>&1 | tee .test.log
	$(ETIME)

.test.log: test

################### Coverage ####################
.PHONY: coverage
coverage: kcov measure upload-coverage

.PHONY: upload-coverage
upload-coverage:
	$(STIME)
	@echo "Uploading coverage to https://codecov.io....."
	@curl -s https://codecov.io/bash | bash
	$(ETIME)

.test_list: .test.log
	@cat .test.log | grep "Running" |cut -f7 -d ' ' > .test_list

.PHONY: measure
measure: .test_list
	$(STIME)
	@echo "Measuring coverage using 'kcov'"
	@for file in `cat .test_list`; do mkdir -p "target/cov/$(basename $$file)"; echo "-------> Testing coverage of $$file"; kcov --include-pattern=$$FLOW_ROOT --exclude-path=flowc/tests,flowr/tests --exclude-region='#[cfg(test)]:#[cfg(testkcovstopmarker)]' "target/cov/$(basename $$file)" $$file; done
	$(ETIME)

.PHONY: kcov
kcov: $(KCOV)
ifeq ($(KCOV),)
	$(STIME)
	@echo "'kcov' is not installed. Building and installing it"
	@echo "Downloading kcov source code"
	@cd target && rm -f target/master.tar.gz && wget -q https://github.com/SimonKagstrom/kcov/archive/master.tar.gz
	@echo "Untarring downloaded kcov tarball"
	@cd target && rm -rf kcov-master && tar xzf master.tar.gz
	@echo "Building kcov from source"
ifeq ($(UNAME), Linux)
	@cd target/kcov-master && rm -rf build && mkdir build && cd build && cmake .. && make && sudo make install
endif
ifeq ($(UNAME), Darwin)
	@echo "Installing 'openssl' and 'binutils' with brew"
	@brew install binutils 2>/dev/null; true
	@# Remove python 2 to avoid dependency issue on osx
	@# https://askubuntu.com/questions/981663/python2-7-broken-by-weakref-import-error-please-help
	@brew remove python@2 --ignore-dependencies 2>/dev/null; true
	@echo "Installing required python packages: 'six'"
	@pip3 -q install six 2>/dev/null
	@echo "Linking openssl to a place where the compiler looks for it"
	@sudo ln -s /usr/local/opt/openssl/include/openssl /usr/local/include 2>/dev/null; true
	@sudo ln -s /usr/local/Cellar/openssl@1.1/1.1.1g/include/openssl /usr/bin/openssl 2>/dev/null; true
	@sudo ln -s /usr/local/opt/openssl/lib/libssl.1.1.1.dylib /usr/local/lib/ 2>/dev/null; true
	@sudo ln -s /usr/local/Cellar/openssl@1.1/1.1.1g/lib/libcrypto.1.1.dylib /usr/local/lib/libcrypto.dylib 2>/dev/null; true
	@#sudo ln -s /usr/local/Cellar/openssl@1.1/1.1.1g/lib/libssl.dylib /usr/local/lib/
	@# Issue with cmake not being able to generage xcode files: "Xcode 1.5 not supported"
	@#cd target/kcov-master && mkdir build && cd build && cmake -G Xcode .. && xcodebuild -configuration Release
	@cd target/kcov-master && mkdir build && cd build && cmake .. && make
	@sudo mv target/kcov-master/build/src/kcov /usr/local/bin/kcov
endif
	@echo "'kcov' installed to `which kcov`, removing build artifacts"
	@rm -rf kcov-master
	@rm -f master.tar.gz*
	$(ETIME)
else
	@echo "'kcov' found at `which kcov`"
endif

#################### FLOW LIBRARIES ####################
.PHONY: flowstdlib
flowstdlib: flowstdlib/manifest.json

flowstdlib/manifest.json: $(FLOWSTDLIB_SOURCES) $(FLOWSTDLIB_TOMLS) $(FLOWSTDLIB_MARKDOWN)
	@mkdir -p target;date '+%s' > target/.flowstdlibtime ; echo "<------ Target '$@' starting"
	@cargo run -p flowc -- -v info -l -g -d flowstdlib
	@read st < target/.flowstdlibtime ; st=$$((`date '+%s'`-$$st)) ; echo "------> Target '$@' done in $$st seconds"

#################### Raspberry Pi ####################
.PHONY: pi
pi:
	@echo "Building flowc for pi in $(PWD)"
# https://hub.docker.com/r/dlecan/rust-crosscompiler-arm
	docker run -it --rm -v $(PWD):/source -v ~/.cargo/git:/root/.cargo/git -v ~/.cargo/registry:/root/.cargo/registry dlecan/rust-crosscompiler-arm:stable
# In case of permissions problems for cargo cache on local machine:
# sudo chown -R `stat -c %u:%g $HOME` $(pwd) ~/.cargo

.PHONY: copy
copy:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/release/flowc andrew@zero-w:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/release/flowr andrew@zero-w:

#################### SAMPLES ####################
# Find all sub-directories under 'samples' and create a list of paths like 'sample/{directory}/test.output' to use for
# make paths - to compile all samples found in there. Avoid files using the filter.
sample_flows := $(patsubst samples/%,samples/%test.output,$(filter %/, $(wildcard samples/*/)))

# This target must be below sample-flows in the Makefile
.PHONY: samples
samples: build flowstdlib
	$(STIME)
	@$(MAKE) $(sample_flows)
	$(ETIME)

samples/%: samples/%/test.err
	$(MAKE) $(@D)/test.output

samples/%/test.output: samples/%/test.input samples/%/test.arguments
	@printf "\tSample '$(@D)'"
	@cat $< | RUST_BACKTRACE=1 cargo run --quiet -p flowr -- --native $(@D)/manifest.json `cat $(@D)/test.arguments` 2> $(@D)/test.err > $@
	@diff $@ $(@D)/expected.output || (ret=$$?; cp $@ $(@D)/failed.output && rm -f $@ && rm -f $(@D)/test.file && exit $$ret)
	@if [ -s $(@D)/expected.file ]; then diff $(@D)/expected.file $(@D)/test.file; fi;
	@if [ -s $(@D)/test.err ]; then (printf " has error output in $(@D)/test.err\n"; exit -1); else printf " has no errors\n"; fi;
	@rm $@ #remove test.output after successful diff so that dependency will cause it to run again next time
# leave test.err for inspection in case of failure

.PHONY: clean-samples
clean-samples:
	$(STIME)
	@find samples -name test.output -exec rm -rf {} + ; true
	@find samples -name test.file -exec rm -rf {} + ; true
	@find samples -name failed.output -exec rm -rf {} + ; true
	$(ETIME)

################# ONLINE SAMPLES ################
.PHONY: online-samples
online-samples:
	$(STIME)
	@echo "Hello" | cargo run --p flowc -- https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml
	$(ETIME)

################# Clean ################
.PHONY: clean
clean:
	@$(MAKE) clean-dumps clean-svgs clean-guide clean-flowstdlib clean-samples
	@cargo clean

.PHONY: clean-flowstdlib
clean-flowstdlib:
	$(STIME)
	@find flowstdlib -name \*.wasm -type f -exec rm -rf {} + ; true
	@rm -f flowstdlib/manifest.json
	$(ETIME)

.PHONY: clean-dumps
clean-dumps:
	$(STIME)
	@find . -name \*.dump -type f -exec rm -rf {} + ; true
	@find . -name \*.dot -type f -exec rm -rf {} + ; true
	$(ETIME)

.PHONY: config
clean-svgs:
	$(STIME)
	@find . -name \*.dot.svg -type f -exec rm -rf {} + ; true
	$(ETIME)

.PHONY: clean-guide
clean-guide:
	$(STIME)
	@rm -rf target/html
	$(ETIME)