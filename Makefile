DOT := $(shell command -v dot 2> /dev/null)
KCOV := $(shell command -v kcov 2> /dev/null)
APTGET := $(shell command -v apt-get 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
STIME = @mkdir -p target;date '+%s' > target/.$@time ; echo "<------ Target '$@' starting"
ETIME = @read st < target/.$@time ; st=$$((`date '+%s'`-$$st)) ; echo "------> Target '$@' done in $$st seconds"
SOURCES = $(shell find . -type f -name \*.rs)
MARKDOWN = $(shell find . -type f -name \*.md)
SAMPLE_FLOWS := $(shell find samples -depth 2 -type f -name \*.toml -not -path "**/flow.toml")
SAMPLE_DOTS := $(patsubst %.toml,%.dot,$(SAMPLE_FLOWS))
SAMPLE_SVGS := $(patsubst %.dot,%.dot.svg,$(SAMPLE_DOTS))
# Find all sub-directories under 'samples' and create a list of paths like 'sample/{directory}/test.output' to use for
# make paths - to compile all samples found in there. Avoid files using the filter.
SAMPLE_OUTPUTS := $(patsubst samples/%,samples/%test.output,$(filter %/, $(wildcard samples/*/)))
FLOWSTDLIB_SOURCES := $(shell find flowstdlib -type f -name \*.rs)
FLOWSTDLIB_TOMLS := $(shell find flowstdlib -type f -name \*.toml)
FLOWSTDLIB_MARKDOWN := $(shell find flowstdlib -type f -name \*.md)
UNAME := $(shell uname)
ONLINE := $(shell ping -q -c 1 -W 1 8.8.8.8 2> /dev/null)
export FLOW_ROOT := $(dir $(realpath $(firstword $(MAKEFILE_LIST))))
export SHELL := /bin/bash

########## Phony targets a user can specify to make life easier #########
.PHONY: all
all: .clippy .build .test .samples .docs
.PHONY: clippy
clippy: .clippy
.PHONY: test
test: .test
.PHONY: build
build: .build
.PHONY: docs
docs: .docs
.PHONY: book
book: target/html/index.html
.PHONY: pages
pages: docs deploy-pages
.PHONY: flowstdlib
flowstdlib: flowstdlib/manifest.json
.PHONY: sample_flows
sample_flows: samples

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
	@brew install gtk+3 glib cairo atk cmake graphviz
	$(ETIME)

.PHONY: config-linux
config-linux:
	$(STIME)
ifneq ($(YUM),)
	@echo "	Installing linux specific dependencies using $(YUM)"
	@sudo yum --color=auto --quiet install curl-devel elfutils-libelf-devel elfutils-devel openssl-devel binutils-devel
	@sudo yum --color=auto --quiet install graphviz gtk3-devel || true
else ifneq ($(APTGET),)
	@echo "	Installing linux specific dependencies using $(APTGET)"
	@sudo apt-get -y install libcurl4-openssl-dev libelf-dev libdw-dev libssl-dev binutils-dev
	@sudo apt-get -y install graphviz libgtk-3-dev || true
else
	@echo "	Neither apt-get nor yum detected for installing linux specific dependencies"
endif
	$(ETIME)

################### Doc ####################
.docs: target/html/index.html target/html/code .trim-docs
	@touch .docs

target/html/index.html: $(MARKDOWN) $(SAMPLE_SVGS)
	@RUST_LOG=info time mdbook build

%.dot: %.output
	@echo Generate a dot for flow $<

%.dot.svg: %.dot
ifeq ($(DOT),)
	@echo "        Install 'graphviz' to be able to generate dot graphs for flows."
else
	@dot -Tsvg -O $<
	@echo "        Generated $@ from $<"
endif

.trim-docs:
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
	@touch .trim-docs

target/html/code: $(SOURCES)
	@cargo doc --workspace --quiet --all-features --no-deps --target-dir=target/html/code

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
.build: $(SOURCES) flowstdlib/manifest.json
	$(STIME)
	@PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:/usr/local/opt/lib/pkgconfig:/usr/local/Cellar/glib/2.62.3/lib/pkgconfig:/usr/lib64/pkgconfig" cargo build
	@touch .build
	$(ETIME)

.clippy: $(SOURCES)
	@cargo clippy -- -D warnings
	@touch .clippy

#################### Tests ####################
.test: $(SOURCES)
	$(STIME)
	@set -o pipefail && cargo test --all-features --workspace --exclude flow_impl_derive --exclude flowide 2>&1 | tee .test.log
	@touch .test
	$(ETIME)

.test.log: .test

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
.samples: .build flowstdlib/manifest.json $(SAMPLE_FLOWS)
	$(STIME)
	@$(MAKE) $(SAMPLE_OUTPUTS)
	$(ETIME)

# If a sample fails then test.output won't be created
samples/%: samples/%/test.err
	$(MAKE) $(@D)/test.output

samples/%/test.output: samples/%/test.input samples/%/test.arguments samples/%/context.toml flowc flowr flowstdlib
	@printf "\tSample '$(@D)'"
	@RUST_BACKTRACE=1 cargo run --quiet -p flowc -- -g -d $(@D) -i $(@D)/test.input -- `cat $(@D)/test.arguments` 2> $(@D)/test.err > $@
	@diff $@ $(@D)/expected.output || (ret=$$?; cp $@ $(@D)/failed.output && rm -f $@ && exit $$ret)
	@if [ -s $(@D)/test.err ]; then (printf " has error output in $(@D)/test.err\n"; exit -1); else printf " has no errors\n"; fi;
	@rm $@ #remove test.output after successful diff so that dependency will cause it to run again next time

.PHONY: clean-samples
clean-samples:
	$(STIME)
	@find samples -name \*.wasm -exec rm -rf {} + ; true
	@find samples -name test.output -exec rm -rf {} + ; true
	@find samples -name failed.output -exec rm -rf {} + ; true
	@find samples -name manifest.json -exec rm -rf {} + ; true
	@find samples -name \*.dump -type f -exec rm -rf {} + ; true
	@find samples -name \*.txt -type f -exec rm -rf {} + ; true
	@find samples -name \*.dot -type f -exec rm -rf {} + ; true
	@find samples -name \*.dot.svg -type f -exec rm -rf {} + ; true
	$(ETIME)

################# ONLINE SAMPLES ################
.PHONY: online-samples
online-samples:
	$(STIME)
	@echo "Hello" | cargo run --p flowc -- https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml
	$(ETIME)

################# Packaging ################
#### Due to dependencies between packages, they need to be published in a given order. Basically this is a DAG and
#### you need to publish from the leaves at the bottom, upwards. I have separated into layers as there are some
#### groups of packages (in same layer) that have same dependencies but are indpendant and they could be published
#### in parallel. But they both need to be published before the next layer up.
#### Level 0 - the root
.PHONY: publish
publish: flowc-publish flowr-publish flowide-publish

#### Level 1 - flowc and flowide - no dependency between them
.PHONY: flowc-publish
flowc-publish: flowr-publish flowrlib-publish provider-publish
	cargo publish --manifest-path=flowc/Cargo.toml

.PHONY: flowide-publish
flowide-publish: flowc-publish flowrlib-publish provider-publish flow-impl-publish flowstdlib-publish
	cargo publish --manifest-path=flowide/Cargo.toml

#### Level 2 - flowr
.PHONY: flowr-publish
flowr-publish: provider-publish flow-impl-publish flowstdlib-publish
	cargo publish --manifest-path=flowr/Cargo.toml

#### Level 3 - provider
.PHONY: provider-publish
provider-publish: flowrlib-publish
	cargo publish --manifest-path=provider/Cargo.toml

#### Level 4 - flowstdlib
.PHONY: flowstdlib-publish
flowstdlib-publish: flow-impl-publish flow-impl-derive-publish flowrlib-publish
	cargo publish --manifest-path=flowstdlib/Cargo.toml

#### Level 5 - flowruntime
.PHONY: flowruntime-publish
flowruntime-publish: flow-impl-publish flowrlib-publish

#### Level 6 - flowrlib
.PHONY: flowrlib-publish
flowrlib-publish: flow-impl-publish
	cargo publish --manifest-path=flowrlib/Cargo.toml

#### Level 7 - flow-impl-publish flow-impl-derive-publish
.PHONY: flow-impl-publish
flow-impl-publish:
	cargo publish --manifest-path=flow_impl/Cargo.toml

.PHONY: flow-impl-derive-publish
flow-impl-derive-publish:
	cargo publish --manifest-path=flow_impl_derive/Cargo.toml

################# Clean ################
.PHONY: clean
clean:
	@$(MAKE) clean-dumps clean-svgs clean-guide clean-flowstdlib clean-samples
	@cargo clean
	@rm -f .clippy .build .test .all

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