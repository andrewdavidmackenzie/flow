DOT := $(shell command -v dot 2> /dev/null)
KCOV := $(shell command -v kcov 2> /dev/null)
APTGET := $(shell command -v apt-get 2> /dev/null)
ZMQ := $(shell command -v brew ls --versions zmq 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
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
	@echo "Detected $(UNAME)"
# Only need to install these dependencies if NOT running in Travis CI - as they will be installed using add-ons there
#ifndef $(CI)
ifeq ($(UNAME), Linux)
	@$(MAKE) config-linux
endif
ifeq ($(UNAME), Darwin)
	@$(MAKE) config-darwin
endif
#endif

.PHONY: common-config
common-config:
	@echo "Installing clippy command using rustup"
	@export PATH="$$PATH:~/.cargo/bin"
	@rustup --quiet component add clippy
	@echo "Installing wasm32 target using rustup"
	@rustup --quiet target add wasm32-unknown-unknown

.PHONY: config-darwin
config-darwin:
	@echo "Installing macos specific dependencies using brew"
ifeq ($(ZMQ),)
	@echo "	Installing zmq"
	@brew install --quiet zmq
else
	@echo "	Detected zmq, skipping install"
endif

.PHONY: config-linux
config-linux:
ifneq ($(YUM),)
	@echo "	Installing linux specific dependencies using $(YUM)"
	@sudo yum install curl-devel elfutils-libelf-devel elfutils-devel openssl-devel binutils-devel zeromq zeromq-devel || true
else ifneq ($(APTGET),)
	@echo "	Installing linux specific dependencies using $(APTGET)"
	@sudo apt-get -y install libcurl4-openssl-dev libelf-dev libdw-dev libssl-dev binutils-dev libzmq3-dev || true
else
	@echo "	Neither apt-get nor yum detected for installing linux specific dependencies"
	@exit 1
endif

################### Doc ####################
.PHONY: docs
docs: build-flowc book code-docs trim-docs

.PHONY: mdbook
mdbook:
	@echo "	Installing mdbook and mdbook-linkcheck using cargo"
	@cargo install mdbook
	@cargo install mdbook-linkcheck

.PHONY: book
book: dot mdbook target/html/index.html

dot:
ifeq ($(DOT),)
	@echo "        Installing 'graphviz' package to be able to convert 'dot' files created by flowc into SVG files for use in docs"
ifeq ($(UNAME), Linux)
ifneq ($(YUM),)
	@sudo yum install graphviz
else ifneq ($(APTGET),)
	@sudo apt-get -y install graphviz
else
	@echo "	Neither apt-get nor yum detected for installing 'graphviz' on linux"
	@exit 1
endif
endif
ifeq ($(UNAME), Darwin)
	@brew install graphviz
endif
else
	@echo "        'dot' command was already installed, skipping 'graphviz' installation"
endif

target/html/index.html: $(MARKDOWN) $(SVGS)
	@RUST_LOG=info time mdbook build

%.dot.svg: %.dot
	@dot -Tsvg -O $<
	@echo "        Generated $@ from $<"

%.dot:

.PHONY: trim-docs
trim-docs:
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

.PHONY: code-docs
code-docs:
	@cargo doc --workspace --quiet --no-deps --target-dir=target/html/code

.PHONY: pages
pages: docs deploy-pages

.PHONY: deploy-pages
deploy-pages:
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

#################### Build ####################
# This is currently needed as the build of the workspace also builds flowstdlib, which requires
# `flowc` binary to have completed first
.PHONY: build-flowc
build-flowc:
	@cargo build -p flowc

.PHONY: build
build: build-flowc
	@PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:/usr/local/opt/lib/pkgconfig:/usr/local/Cellar/glib/2.62.3/lib/pkgconfig:/usr/lib64/pkgconfig" cargo build --workspace

build-all-features: build-flowc
	cd flowcore && cargo build-all-features
	cd flowr && cargo build-all-features
	cd flowc && cargo build-all-features

.PHONY: clippy
clippy: build-flowc
	@cargo clippy -- -D warnings

#################### Tests ####################
.PHONY: test
test: build-flowc
	@set -o pipefail && cargo test --workspace --exclude flow_impl_derive -- --test-threads 1 2>&1 | tee .test.log

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
	@echo "Uploading coverage to https://codecov.io....."
	@curl -s https://codecov.io/bash | bash

.test_list: .test.log
	@cat .test.log | grep "Running" |cut -f7 -d ' ' > .test_list

.PHONY: measure
measure: .test_list
	@echo "Measuring coverage using 'kcov'"
	@for file in `cat .test_list`; do mkdir -p "target/cov/$(basename $$file)"; echo "-------> Testing coverage of $$file"; kcov --exclude-path=flowc/tests,flowr/tests --exclude-region='#[cfg(test)]:#[cfg(testkcovstopmarker)]' "target/cov/$(basename $$file)" $$file; done

.PHONY: kcov
kcov: $(KCOV)
ifeq ($(KCOV),)
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
	@find . -name manifest.json -type f -exec rm -rf {} + ; true
	@find . -name test.output -type f -exec rm -rf {} + ; true
	@find . -name test.err -type f -exec rm -rf {} + ; true
	@find . -name \*.wasm -type f -exec rm -rf {} + ; true
	@rm -rf target/html
	@find . -name \*.dump -type f -exec rm -rf {} + ; true
	@cargo clean