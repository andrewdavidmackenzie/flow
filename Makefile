APTGET := $(shell command -v apt-get 2> /dev/null)
ZMQ := $(shell brew ls --versions zmq 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
DNF := $(shell command -v dnf 2> /dev/null)
BREW := $(shell command -v brew 2> /dev/null)
ONLINE := $(shell ping -c 1 https://raw.githubusercontent.com 2> /dev/null)
export SHELL := /bin/bash

ifeq ($(ONLINE),true)
features := --features "wasm" "online_tests"
else
features := --features "wasm"
endif

.PHONY: all
all: clippy build test docs

.PHONY: config
config:
	@echo "Installing clippy command using rustup"
	@export PATH="$$PATH:~/.cargo/bin"
	@echo "Installing nightly with rustup for clippy nightly and coverage measurement"
	@rustup install nightly
	@rustup --quiet component add clippy
	@echo "Installing wasm32 target using rustup"
	@rustup --quiet target add wasm32-unknown-unknown
ifneq ($(BREW),)
	@echo "Installing Mac OS X specific dependencies using $(BREW)"
	@brew install --quiet zmq graphviz binaryen
endif
ifneq ($(DNF),)
	@echo "Installing linux specific dependencies using $(DNF)"
	@echo "To build OpenSSL you need perl installed"
	@sudo dnf install perl
	@sudo dnf install curl-devel elfutils-libelf-devel elfutils-devel openssl openssl-devel binutils-devel || true
	@sudo dnf install zeromq zeromq-devel graphviz binaryen || true
endif
ifneq ($(YUM),)
	@echo "Installing linux specific dependencies using $(YUM)"
	@echo "To build OpenSSL you need perl installed"
	@sudo yum install perl
	@sudo yum install curl-devel elfutils-libelf-devel elfutils-devel openssl openssl-devel binutils-devel || true
	@sudo yum install zeromq zeromq-devel graphviz binaryen || true
endif
ifneq ($(APTGET),)
	@echo "Installing linux specific dependencies using $(APTGET)"
	@echo "To build OpenSSL you need perl installed"
	@sudo apt-get install perl
	@sudo apt-get -y install libcurl4-openssl-dev libelf-dev libdw-dev libssl-dev binutils-dev || true
	@sudo apt-get -y install libzmq3-dev graphviz binaryen || true
endif
	@echo "	Installing mdbook and mdbook-linkcheck using cargo"
	@cargo install mdbook
	@cargo install mdbook-linkcheck
	@echo "installing wasm optimization tools"
	@cargo install wasm-gc wasm-snip

.PHONY: clean
clean:
	@echo "clean<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo clean
	@find samples -name "*.wasm" | xargs rm -f

.PHONY: install-flowc
install-flowc:
	@echo "install-flowc<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo install --path flowc

.PHONY: clippy
clippy: install-flowc
	@echo "clippy<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo clippy -- -D warnings

.PHONY: build
build: install-flowc
	@echo "build<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo build $(features)

.PHONY: test
test: install-flowc
	@echo "test<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo test $(features)

.PHONY: docs
docs:
	@echo "docs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo doc --no-deps --target-dir=target/html/code
	@mdbook build
	@find target/html -name .git | xargs rm -rf {}
	@find target/html -name .github | xargs rm -rf {}
	@find target/html -name .gitignore | xargs rm -rf {}
	@find target/html -name .idea | xargs rm -rf {}
	@find target/html -name \*.iml | xargs rm -rf {}
	@find target/html -name \*.toml | xargs rm -rf {}
	@find target/html -name \*.profraw | xargs rm -rf {}
	@find target/html -name manifest.json | xargs rm -rf {}
	@find target/html -name manifest.rs | xargs rm -rf {}
	@find target/html -name target -type d | xargs rm -rf {}
	@find target/html -name test.err | xargs rm -rf {}
	@find target/html -name test.input | xargs rm -rf {}
	@find target/html -name test.arguments | xargs rm -rf {}
	@find target/html -name test.output | xargs rm -rf {}
	@find target/html -name test.file | xargs rm -rf {}
	@find target/html -name expected.file | xargs rm -rf {}
	@find target/html -name expected.output | xargs rm -rf {}
	@find target/html -name \*.rs | xargs rm -rf {}
	@find target/html -name \*.dump | xargs rm -rf {}
	@find target/html -name \*.dot | xargs rm -rf {}
	@find target/html -name \*.wasm | xargs rm -rf {}
	@find target/html -name \*.lock  | xargs rm -rf {}
	@rm -rf target/html/.mdbookignore
	@rm -rf target/html/.DS_Store
	@rm -rf target/html/book.toml
	@rm -rf target/html/codecov.yml
	@rm -rf target/html/flowc/tests/test-flows
	@rm -rf target/html/flowc/tests/test-functions/stdio
	@rm -rf target/html/flowc/tests/test_libs
	@rm -rf target/html/code/debug
	@rm -rf target/html/Makefile
	@find target/html -depth -type d -empty -delete
