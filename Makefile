APTGET := $(shell command -v apt-get 2> /dev/null)
ZMQ := $(shell brew ls --versions zmq 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
BREW := $(shell command -v brew 2> /dev/null)
export SHELL := /bin/bash

.PHONY: all
all: clippy build test docs

.PHONY: config
config:
	@echo "Installing clippy command using rustup"
	@export PATH="$$PATH:~/.cargo/bin"
	@rustup --quiet component add clippy
	@echo "Installing wasm32 target using rustup"
	@rustup --quiet target add wasm32-unknown-unknown
	@echo "	Installing mdbook and mdbook-linkcheck using cargo"
	@cargo install mdbook
	@cargo install mdbook-linkcheck
ifneq ($(BREW),)
	@echo "Installing Mac OS X specific dependencies using $(BREW)"
	@brew install --quiet zmq graphviz
endif
ifneq ($(YUM),)
	@echo "Installing linux specific dependencies using $(YUM)"
	@echo "To build OpenSSL you need perl installed"
	@sudo yum install perl
	@sudo yum install curl-devel elfutils-libelf-devel elfutils-devel openssl-devel binutils-devel || true
	@sudo yum install zeromq zeromq-devel graphviz || true
endif
ifneq ($(APTGET),)
	@echo "Installing linux specific dependencies using $(APTGET)"
	@echo "To build OpenSSL you need perl installed"
	@sudo apt-get install perl
	@sudo apt-get -y install libcurl4-openssl-dev libelf-dev libdw-dev libssl-dev binutils-dev || true
	@sudo apt-get -y install libzmq3-dev graphviz || true
endif

.PHONY: docs
docs:
	@cargo doc --no-deps --target-dir=target/html/code
	@mdbook build

.PHONY: build-flowc
build-flowc:
	@cargo build -p flowc

.PHONY: compile-flowstdlib
compile-flowstdlib: build-flowc
	@cargo run -p flowc -- -l flowstdlib

.PHONY: build
build: build-flowc compile-flowstdlib
	@cargo build

.PHONY: clippy
clippy: build-flowc compile-flowstdlib
	@cargo clippy -- -D warnings

.PHONY: test
test: build-flowc compile-flowstdlib
	@cargo test

.PHONY: clean
clean:
	@cargo clean
	@find . -name \*.wasm -exec rm {} \;
	@rm -f flowstdlib/manifest.json flowstdlib/lib.rs