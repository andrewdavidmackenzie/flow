APTGET := $(shell command -v apt-get 2> /dev/null)
ZMQ := $(shell brew ls --versions zmq 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
DNF := $(shell command -v dnf 2> /dev/null)
BREW := $(shell command -v brew 2> /dev/null)
RUSTUP := $(shell command -v rustup 2> /dev/null)
CODESIGN := $(shell command -v codesign 2> /dev/null) # Detect codesigning app on mac to avoid security dialogs
$(eval SELFCERT = $(shell security find-certificate -c "self" 2>&1 | grep "self")) # Detect codesigning app on mac to avoid security dialogs

export SHELL := /bin/bash
export PATH := $(PWD)/target/debug:$(PWD)/target/release:$(PATH)

ifeq ($(FLOW_LIB_PATH),)
  export FLOW_LIB_PATH := $(HOME)/.flow/lib
endif

ifeq ($(FLOW_CONTEXT_ROOT),)
  export FLOW_CONTEXT_ROOT := $(PWD)/flowr/src/bin/flowrcli/context
endif

.PHONY: all
all: clean-start build clippy test docs

.PHONY: clean-start
clean-start:
	@find . -name "*.profraw"  | xargs rm -rf {}

.PHONY: rustup
rustup:
ifeq ($(RUSTUP),)
	@echo "Installing rustup"
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
endif

.PHONY: config
config: rustup
	@export PATH="$$PATH:~/.cargo/bin"
	@echo "Installing clippy command using rustup"
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

.PHONY: clean_examples
clean_examples:
	@find flowr/examples -name manifest.json | xargs rm -rf
	@find flowr/examples -name \*.dot | xargs rm -rf
	@find flowr/examples -name \*.dot.svg | xargs rm -rf
	@find flowr/examples -name \*.wasm | xargs rm -rf

.PHONY: clean
clean: clean_examples
	@echo "clean<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo clean
	@find . -name target -type d | xargs rm -rf
	@rm -rf $(HOME)/.flow/lib/flowstdlib

.PHONY: build
build:
	@echo "build<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo build -p flowc # flowc binary used to compile flowstdlib and examples so needed first
	@cargo build -p flowstdlib # Used by examples so needed first
	@cargo build
	@cargo build --examples

.PHONY: clippy
clippy: build
	@echo "clippy<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo clippy --tests --all-features -- -D warnings

.PHONY: test
test: build
	@echo "test<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
ifneq ($(CODESIGN),)
	@echo "Code signing tool \"codesign\" detected"
ifneq ($(SELFCERT),)
	@echo "Self-signing certificate called \"self\" found"
	@cargo test --no-run
	@find target -name "flow*" -perm +111 -type f | xargs codesign -s self || true
endif
endif
	@cargo test
	@cargo test --examples

.PHONY: coverage
coverage: clean-start
	@echo "coverage<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@find . -name "*.profraw"  | xargs rm -rf {} # Remove old coverage measurements
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo build -p flowc # Used to compile flowstdlib and examples, so needed first
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo build -p flowstdlib # Used by examples so needed first
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo build
ifeq ($(CODESIGN),)
	find target -perm +111 -type f | xargs codesign -fs self
endif
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo clippy --tests -- -D warnings
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo test
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo doc --no-deps --target-dir=target/html/code
	@echo "Gathering coverage information"
	@grcov . --binary-path target/debug/ -s . -t lcov --branch --ignore-not-existing --ignore "/*" -o coverage.info
	@lcov --remove coverage.info '/Applications/*' 'target/debug/build/**' 'target/release/build/**' '/usr*' '**/errors.rs' '**/build.rs' '*tests/*' -o coverage.info
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
code-docs: build
	@echo "code-docs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo doc --no-deps --target-dir=target/html/code

.PHONE: copy-svgs
copy-svgs:
	@echo "copy-svgs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@for i in $(shell find flowr/examples -name '*.dot.svg' ); do \
      cp $$i target/html/$$i; \
    done
	@for i in $(shell cd $$HOME/.flow/lib/flowstdlib && find . -name '*.dot.svg' ); do \
      cp $$HOME/.flow/lib/flowstdlib/$$i target/html/flowstdlib/src/$$i; \
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
	@find target/html -name \*.rs  | xargs rm -rf {}
	@find target/html -name Cargo.toml  | xargs rm -rf {}
	@find target/html -name src -type d | xargs rm -rf {}
	@rm -rf target/html/.mdbookignore
	@rm -rf target/html/.DS_Store
	@rm -rf target/html/book.toml
	@rm -rf target/html/codecov.yml
	@rm -rf target/html/flowc/tests/test-flows
	@rm -rf target/html/flowc/tests/test-functions/stdio
	@rm -rf target/html/flowc/tests/test_libs
	@rm -rf target/html/code/debug
	@rm -rf target/html/code/release
	@rm -rf target/html/Makefile
	@rm -rf target/html/.nojekyll
	@rm -rf target/html/coverage.info
	@rm -rf target/html/flowr/examples/Cargo.toml
	@rm -rf target/html/flowr/examples/mandlebrot/project
	@find target/html -depth -type d -empty -delete

.PHONY: release
release:
	cargo release --no-verify --workspace --execute minor