APTGET := $(shell command -v apt-get 2> /dev/null)
ZMQ := $(shell brew ls --versions zmq 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
DNF := $(shell command -v dnf 2> /dev/null)
BREW := $(shell command -v brew 2> /dev/null)
export SHELL := /bin/bash
export PATH := $(PWD)/target/debug:$(PWD)/flowrex/target/debug:$(PATH)

features := --features "wasm"

ifeq ($(FLOW_LIB_PATH),)
  export FLOW_LIB_PATH := $(PWD)/target
endif

ifeq ($(FLOW_CONTEXT_ROOT),)
  export FLOW_CONTEXT_ROOT := $(PWD)/flowr/src/cli
endif

.PHONY: all
all: clean-start clippy build test docs

.PHONY: clean-start
clean-start:
	@find . -name "*.profraw"  | xargs rm -rf {}

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
	@find . -name target -type d | xargs rm -rf

.PHONY: build-binaries
build-binaries: build-flowc build-flowr build-flowrex
	@echo "binaries-built<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"

.PHONY: build-flowc
build-flowc:
	@echo "build-flowc<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo build -p flowc

.PHONY: build-flowr
build-flowr:
	@echo "build-flowr<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo build -p flowr

.PHONY: build-flowrex
build-flowrex:
	@echo "build-flowrex<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo build --manifest-path flowrex/Cargo.toml

.PHONY: clippy
clippy: build-binaries
	@echo "clippy<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo clippy --tests --all-features -- -D warnings

.PHONY: build
build: build-binaries
	@echo "build<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo build $(features)
	@cargo build --manifest-path flowrex/Cargo.toml

.PHONY: test
test: build-binaries
	@echo "test<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo test $(features)

.PHONY: coverage
coverage: clean-start build-binaries
	@echo "coverage<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@find . -name "*.profraw"  | xargs rm -rf {}
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo build -p flowc
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo build -p flowr
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo build --manifest-path flowrex/Cargo.toml
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo clippy --tests -- -D warnings
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo build $(features)
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo test $(features)
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo doc --no-deps --target-dir=target/html/code
	@echo "Gathering coverage information"
	@grcov . --binary-path target/debug/ -s . -t lcov --branch --ignore-not-existing --ignore "/*" -o coverage.info
	@lcov --remove coverage.info '/Applications/*' 'target/debug/build/**' '/usr*' '**/errors.rs' '**/build.rs' '*tests/*' -o coverage.info
	@find . -name "*.profraw" | xargs rm -f
	@echo "Generating coverage report"
	@genhtml -o target/coverage --quiet coverage.info
	@echo "View coverage report using 'open target/coverage/index.html'"

.PHONY: docs
docs: generate-docs copy-svgs trim-docs

.PHONY: generate-docs
generate-docs: build-book code-docs
	@echo "generate-docs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"

.PHONY: build-book
build-book:
	@echo "build-book<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@mdbook build

.PHONY: code-docs
code-docs: build-flowc
	@echo "code-docs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo doc --no-deps --target-dir=target/html/code

.PHONE: copy-svgs
copy-svgs:
	@echo "copy-svgs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@for i in $(shell cd target/flowsamples && find . -name '*.dot.svg' ); do \
      cp target/flowsamples/$$i target/html/flowsamples/$$i; \
    done
	@for i in $(shell cd target/flowstdlib && find . -name '*.dot.svg' ); do \
      cp target/flowstdlib/$$i target/html/flowstdlib/src/$$i; \
    done

.PHONY: trim-docs
trim-docs:
	@echo "trim-docs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@find target/html -name .git | xargs rm -rf {}
	@rm -rf target/html/.git
	@find target/html -name .github | xargs rm -rf {}
	@rm -rf target/html/.github
	@find target/html -name .gitignore | xargs rm -rf {}
	@find target/html -name .idea | xargs rm -rf {}
	@rm -rf target/html/.idea
	@find target/html -name \*.iml | xargs rm -rf {}
	@find target/html -name \*.toml | xargs rm -rf {}
	@find target/html -name \*.profraw | xargs rm -rf {}
	@find target/html -name manifest.json | xargs rm -rf {}
	@find target/html -name manifest.rs | xargs rm -rf {}
	@find target/html -name target -type d | xargs rm -rf {}
	@find target/html -name test.err | xargs rm -rf {}
	@find target/html -name test.stdout | xargs rm -rf {}
	@find target/html -name test.file | xargs rm -rf {}
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
	@rm -rf target/html/.nojekyll
	@rm -rf target/html/coverage.info
	@rm -rf target/html/flowsamples/build.rs
	@rm -rf target/html/flowsamples/main.rs
	@rm -rf target/html/flowsamples/Cargo.toml
	@rm -rf target/html/flowsamples/mandlebrot/project
	@find target/html -depth -type d -empty -delete

.PHONY: release
release:
	cargo release --no-verify --execute minor
	echo "Use 'cargo release --no-verify --execute minor --manifest-path=flowrex/Cargo.toml' to release flowrex after updating dependencies"
