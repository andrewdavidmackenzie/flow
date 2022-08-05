APTGET := $(shell command -v apt-get 2> /dev/null)
ZMQ := $(shell brew ls --versions zmq 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
DNF := $(shell command -v dnf 2> /dev/null)
BREW := $(shell command -v brew 2> /dev/null)
ONLINE := $(shell ping -c 1 https://raw.githubusercontent.com > /dev/null 2>&1 ; echo $$?)
export SHELL := /bin/bash

ifeq ($(ONLINE),0)
features := --features "wasm","online_tests"
else
features := --features "wasm"
endif

ifeq ($(FLOW_LIB_PATH),)
  $(warning FLOW_LIB_PATH is not set. This maybe needed for builds and test and packaging to succeed.\
  A suggested value for development would be '$(PWD)')
endif

ifeq ($(FLOW_CONTEXT_ROOT),)
  $(warning FLOW_CONTEXT_ROOT is not set. This maybe needed for builds and test and packaging to succeed.\
  A suggested value for development would be '$(PWD)/flowr/src/cli')
endif

.PHONY: all
all: online clippy build test docs trim-docs

.PHONY: online
online:
ifeq ($(ONLINE),0)
	@echo "ONLINE, so including 'online_tests' feature"
else
	@echo "Not ONLINE, so not including 'online_tests' feature"
endif

# NOTE: I had some link problems with the flowmacro crate on _my_ mac, which was solved using zld
# as per this post https://dsincl12.medium.com/speed-up-your-rust-compiler-macos-d9fbe0f32dbc
.PHONY: config
config:
	@echo "Installing clippy command using rustup"
	@export PATH="$$PATH:~/.cargo/bin"
	@echo "Installing nightly with rustup for clippy nightly and coverage measurement"
	@rustup install nightly
	@rustup --quiet component add clippy
	@echo "Installing wasm32 target using rustup"
	@rustup --quiet target add wasm32-unknown-unknown
	@echo "Installing llvm-tools-preview for coverage"
	@rustup component add llvm-tools-preview
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

.PHONY: install-flow
install-flow:
	@echo "install-flow<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo install --path flowc
	@cargo install --path flowr

.PHONY: clippy
clippy: install-flow
	@echo "clippy<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo clippy --tests -- -D warnings

.PHONY: build
build: install-flow
	@echo "build<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo build $(features)

.PHONY: test
test: install-flow
	@echo "test<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo test $(features)

.PHONY: coverage
coverage: install-flow
	@echo "coverage<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@find . -name "*.profraw"  | xargs rm -rf {}
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo test $(features)
	@echo "Gathering covering information"
	@grcov . --binary-path target/debug/ -s . -t lcov --branch --ignore-not-existing --ignore "/*" -o coverage.info
	@lcov --remove coverage.info '/Applications/*' '/usr*' '**/errors.rs' '*tests/*' -o coverage.info
	@find . -name "*.profraw" | xargs rm -f
	@echo "Generating coverage report in './target/coverage/index.html'"
	@genhtml -o target/coverage --quiet coverage.info

.PHONY: docs
docs:
	@echo "docs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo doc --no-deps --target-dir=target/html/code
	@mdbook build

.PHONY: trim-docs
trim-docs:
	@echo "trim-docs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
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
	@find target/html -name \*.profraw  | xargs rm -rf {}
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

.PHONY: publish
publish:
	@cargo build --no-default-features
	@cargo ws publish