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
ifneq ($(BREW),)
	@echo "Installing Mac OS X specific dependencies using $(BREW)"
	@echo "	Installing zmq"
	@brew install --quiet zmq graphviz
endif
ifneq ($(YUM),)
	@echo "	Installing mdbook and mdbook-linkcheck using cargo"
	@cargo install mdbook
	@cargo install mdbook-linkcheck
	@echo "Installing linux specific dependencies using $(YUM)"
	@sudo yum install curl-devel elfutils-libelf-devel elfutils-devel openssl-devel binutils-devel || true
	@sudo yum install zeromq zeromq-devel graphviz || true
endif
ifneq ($(APTGET),)
	@echo "	Installing mdbook and mdbook-linkcheck using cargo"
	@cargo install mdbook
	@cargo install mdbook-linkcheck
	@echo "Installing linux specific dependencies using $(APTGET)"
	@sudo apt-get -y install libcurl4-openssl-dev libelf-dev libdw-dev libssl-dev binutils-dev || true
	@sudo apt-get -y install libzmq3-dev graphviz || true
endif

.PHONY: docs
docs: build-flowc book code-docs trim-docs

.PHONY: book
book:
	@mdbook build

.PHONY: code-docs
code-docs:
	@cargo doc --workspace --quiet --no-deps --target-dir=target/html/code

.PHONY: trim-docs
trim-docs:
	@find target/html -name target -type d | xargs rm -rf {}
	@find target/html -name .idea | xargs rm -rf {}
	@find target/html -name \*.iml | xargs rm -rf {}
	@find target/html -name .git | xargs rm -rf {}
	@find target/html -name Cargo.toml | xargs rm -rf {}
	@find target/html -name manifest.json | xargs rm -rf {}
	@find target/html -name test.err | xargs rm -rf {}
	@find target/html -name test.input | xargs rm -rf {}
	@find target/html -name test.arguments | xargs rm -rf {}
	@find target/html -name test.output | xargs rm -rf {}
	@find target/html -name expected.output | xargs rm -rf {}
	@find target/html -name flow.toml | xargs rm -rf {}
	@find target/html -name \*.rs | xargs rm -rf {}
	@find target/html -name \*.dump | xargs rm -rf {}
	@find target/html -name \*.dot | xargs rm -rf {}
	@find target/html -name \*.wasm | xargs rm -rf {}
	@find target/html -name \*.lock  | xargs rm -rf {}
	@cd target/html && rm -f Makefile .crates.toml .DS_Store .mdbookignore .travis.yml codecov.yml
	@rm -rf target/html/flowc/tests/test-flows
	@rm -rf target/html/flowc/tests/test-libs
	@rm -rf target/html/code/debug
	@find target/html -depth -type d -empty -delete

.PHONY: build-flowc
build-flowc:
	@cargo build -p flowc

.PHONY: build
build: build-flowc
	@cargo build

.PHONY: clippy
clippy: build-flowc
	@cargo clippy -- -D warnings

.PHONY: test
test: build-flowc
	@set -o pipefail && cargo test -- --test-threads 1 2>&1

.PHONY: clean
clean:
	@find . -name \*.dot.svg -type f -exec rm -rf {} + ; true
	@find . -name \*.dot -type f -exec rm -rf {} + ; true
	@find . -name \*.profraw -type f -exec rm -rf {} + ; true
	@find . -name manifest.json -type f -exec rm -rf {} + ; true
	@find . -name test.output -type f -exec rm -rf {} + ; true
	@find . -name test.err -type f -exec rm -rf {} + ; true
	@find . -name \*.wasm -type f -exec rm -rf {} + ; true
	@rm -rf target/html
	@find . -name \*.dump -type f -exec rm -rf {} + ; true
	@cargo clean
