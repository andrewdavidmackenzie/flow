APTGET := $(shell command -v apt-get 2> /dev/null)
ZMQ := $(shell brew ls --versions zmq 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
DNF := $(shell command -v dnf 2> /dev/null)
BREW := $(shell command -v brew 2> /dev/null)
ONLINE := $(shell ping -c 1 github.com > /dev/null 2>&1 ; echo $$?)
export SHELL := /bin/bash

ifeq ($(ONLINE),0)
features := --features "wasm","online_tests"
cargo_options :=
else
features := --features "wasm"
cargo_options := --offline
endif

ifeq ($(FLOW_LIB_PATH),)
  $(warning FLOW_LIB_PATH is not set. This maybe needed for builds and test and packaging to succeed.\
  A suggested value for development would be '$(PWD)/target')
endif

ifeq ($(FLOW_CONTEXT_ROOT),)
  $(warning FLOW_CONTEXT_ROOT is not set. This maybe needed for builds and test and packaging to succeed.\
  A suggested value for development would be '$(PWD)/flowr/src/cli')
endif

.PHONY: all
all: online clippy build test docs

.PHONY: online
online:
ifeq ($(ONLINE),0)
	@echo "ONLINE, so including 'online_tests' feature"
else
	@echo "Not ONLINE, so not including 'online_tests' feature"
endif

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
	@cargo install --path flowc $(cargo_options)
	@cargo install --path flowr $(cargo_options)
	@cargo install --path flowrex $(cargo_options)

.PHONY: clippy
clippy: install-flow
	@echo "clippy<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo clippy --tests --all-features -- -D warnings

.PHONY: build
build: install-flow
	@echo "build<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo build $(features) $(cargo_options)
	@cargo build $(cargo_options) --manifest-path flowrex/Cargo.toml

.PHONY: test
test: install-flow
	@echo "test<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo test $(features) $(cargo_options)

.PHONY: coverage
coverage: install-flow
	@echo "coverage<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@find . -name "*.profraw"  | xargs rm -rf {}
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo test $(features) $(cargo_options)
	@echo "Gathering covering information"
	@grcov . --binary-path target/debug/ -s . -t lcov --branch --ignore-not-existing --ignore "/*" -o coverage.info
	@lcov --remove coverage.info '/Applications/*' '/usr*' '**/errors.rs' '**/build.rs' '*tests/*' -o coverage.info
	@find . -name "*.profraw" | xargs rm -f
	@echo "Generating coverage report in './target/coverage/index.html'"
	@genhtml -o target/coverage --quiet coverage.info

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
code-docs:
	@echo "code-docs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo doc --no-deps --target-dir=target/html/code $(cargo_options)

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

.PHONY: publish
publish:
	# "|| true" is to continue publishing other crates when one has been published already and fails
	@cd flowcore;cargo publish || true
	@cd flowmacro;cargo publish || true
	@cd flowrlib;cargo publish || true
	@cd flowc;cargo publish || true
	# See https://github.com/andrewdavidmackenzie/flow/issues/1517 to understand --no-verify flag
	# used on all builds that build flowstdlib or flowsamples due to Cargo.toml of supplied implementations
	@cd flowstdlib;cargo publish --no-verify || true
	@cd flowr;cargo publish --no-verify || true
	@cd flowrex;cargo publish --no-verify || true
	@cd flowsamples;cargo publish --no-verify || true