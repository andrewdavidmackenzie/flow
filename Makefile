DOT := $(shell command -v dot 2> /dev/null)
KCOV := $(shell command -v kcov 2> /dev/null)
APTGET := $(shell command -v apt-get 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
STIME = @mkdir -p target;date '+%s' > target/.$@time ; echo "<------ Target '$@' starting"
ETIME = @read st < target/.$@time ; st=$$((`date '+%s'`-$$st)) ; echo "------> Target '$@' done in $$st seconds"
MARKDOWN = $(shell find . -type f -name \*.md)
DOTS = $(shell find . -type f -name \*.dot)
SVGS = $(patsubst %.dot,%.dot.svg,$(DOTS))
UNAME := $(shell uname)
ONLINE := $(shell ping -q -c 1 -W 1 8.8.8.8 2> /dev/null)
#TARGET_ARCH := armv7-unknown-linux-musleabihf # For arm7 targets
#TARGET_TOOLCHAIN := ../arm-linux-musleabihf-cross/armv7-linux-musleabihf  # For arm7 targets
TARGET_ARCH := arm-unknown-linux-musleabihf # For piZero target
#TARGET_TOOLCHAIN := ../arm-linux-musleabihf-cross/arm-linux-musleabihf  # For piZero target
REMOTE_HOST := andrew@pi
REMOTE_DIR := /home/andrew/bin
export SHELL := /bin/bash

.PHONY: all
all: clippy build test docs

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
	@brew install cmake graphviz zmq
	$(ETIME)

.PHONY: config-linux
config-linux:
	$(STIME)
ifneq ($(YUM),)
	@echo "	Installing linux specific dependencies using $(YUM)"
	@sudo yum --color=auto --quiet install curl-devel elfutils-libelf-devel elfutils-devel openssl-devel binutils-devel || true
	@sudo yum --color=auto --quiet install graphviz zeromq zeromq-devel || true
else ifneq ($(APTGET),)
	@echo "	Installing linux specific dependencies using $(APTGET)"
	@sudo apt-get -y install libcurl4-openssl-dev libelf-dev libdw-dev libssl-dev binutils-dev || true
	@sudo apt-get -y install graphviz libzmq3-dev || true
else
	@echo "	Neither apt-get nor yum detected for installing linux specific dependencies"
endif
	$(ETIME)

################### Doc ####################
.PHONY: docs
docs: build-flowc book code-docs trim-docs

.PHONY: book
book: target/html/index.html

target/html/index.html: $(MARKDOWN) $(SVGS)
	@RUST_LOG=info time mdbook build

%.dot.svg: %.dot
ifeq ($(DOT),)
	@echo "        Install 'graphviz' to be able to convert 'dot' files created by flowc into SVG files for use in docs"
else
	@dot -Tsvg -O $<
	@echo "        Generated $@ from $<"
endif

# This target can be used to manually generate new SVG files from dot files
.PHONY: dot-graphs
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
code-docs:
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
# This is currently needed as the build of the workspace also builds flowstdlib, which requires
# `flowc` binary to have completed first
.PHONY: build-flowc
build-flowc:
	$(STIME)
	@cargo build -p flowc
	$(ETIME)

.PHONY: build
build: build-flowc
	$(STIME)
	@PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:/usr/local/opt/lib/pkgconfig:/usr/local/Cellar/glib/2.62.3/lib/pkgconfig:/usr/lib64/pkgconfig" cargo build --workspace
	$(ETIME)

build-all-features: build-flowc
	cd flowcore && cargo build-all-features
	cd flowr && cargo build-all-features
	cd flowc && cargo build-all-features

.PHONY: clippy
clippy: build-flowc
	$(STIME)
	@cargo clippy -- -D warnings
	$(ETIME)

#################### Tests ####################
.PHONY: test
test: build-flowc
	$(STIME)
	@set -o pipefail && cargo test --workspace --exclude flow_impl_derive -- --test-threads 1 2>&1 | tee .test.log
	$(ETIME)

.test.log: test

test-all-features: build-flowc
	cd flowcore && cargo test-all-features
	cd flowr && cargo test-all-features
	cd flowc && cargo test-all-features

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
	@for file in `cat .test_list`; do mkdir -p "target/cov/$(basename $$file)"; echo "-------> Testing coverage of $$file"; kcov --exclude-path=flowc/tests,flowr/tests --exclude-region='#[cfg(test)]:#[cfg(testkcovstopmarker)]' "target/cov/$(basename $$file)" $$file; done
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
	@# Issue with cmake not being able to generate xcode files: "Xcode 1.5 not supported"
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

#################### Raspberry Pi ####################
target/${TARGET_ARCH}/debug/flowr:
	cargo build --target=${TARGET_ARCH} -p flowr

.PHONY: pi
pi: target/${TARGET_ARCH}/debug/flowr
	@echo "Building flowc for pi in $(PWD)"
	rsync -azh $< ${REMOTE_HOST}:${REMOTE_DIR}/hello

.PHONY: copy
copy:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/release/flowc andrew@zero-w:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/release/flowr andrew@zero-w:

################# Clean ################
.PHONY: clean
clean:
	@find . -name \*.dot.svg -type f -exec rm -rf {} + ; true
	@find . -name \*.dot -type f -exec rm -rf {} + ; true
	@rm -rf target/html
	@find . -name \*.dump -type f -exec rm -rf {} + ; true
	@find . -name \*.dot -type f -exec rm -rf {} + ; true
	@cargo clean