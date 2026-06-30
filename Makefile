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

.PHONY: all
all: clean-start build clippy test book

.PHONY: clean-start
clean-start:
	@find . -name "*.profraw"  | xargs rm -rf {}

.PHONY: rustup
rustup:
ifeq ($(RUSTUP),)
	@echo "Installing rustup"
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
endif

#NOTE: Linux distros may also have brew installed - so I put it last. WIll probably only be used for mac
.PHONY: config
config: rustup
	@export PATH="$$PATH:~/.cargo/bin"
	@echo "Installing stable toolchain with rustup"
	@echo "Installing clippy component using rustup"
	@rustup --quiet component add clippy
	@echo "Installing wasm32 target using rustup"
	@rustup --quiet target add wasm32-unknown-unknown
	@echo "Installing llvm-tools-preview component for coverage"
	@rustup component add llvm-tools-preview
ifneq ($(DNF),)
	@echo "Installing dependencies using $(DNF)"
	@echo "To build OpenSSL you need perl installed"
	@sudo dnf install perl
	@sudo dnf install curl-devel elfutils-libelf-devel elfutils-devel openssl openssl-devel binutils-devel || true
	@sudo dnf install zeromq zeromq-devel binaryen lcov || true
else ifneq ($(YUM),)
	@echo "Installing dependencies using $(YUM)"
	@echo "To build OpenSSL you need perl installed"
	@sudo yum install perl
	@sudo yum install curl-devel elfutils-libelf-devel elfutils-devel openssl openssl-devel binutils-devel || true
	@sudo yum install zeromq zeromq-devel binaryen lcov || true
else ifneq ($(APTGET),)
	@echo "Installing dependencies using $(APTGET)"
	@echo "To build OpenSSL you need perl installed"
	@sudo apt-get install perl
	@sudo apt-get -y install libcurl4-openssl-dev libelf-dev libdw-dev libssl-dev binutils-dev || true
	@sudo apt-get -y install libzmq3-dev binaryen lcov || true
	@wget https://github.com/WebAssembly/binaryen/releases/download/version_116/binaryen-version_116-x86_64-linux.tar.gz
	@tar -xvzf binaryen-version_116-x86_64-linux.tar.gz
	@sudo cp binaryen-version_116/bin/* /bin/
	@rm -rf binaryen-version_116
	@rm binaryen-version_116-x86_64-linux.tar.gz
else ifneq ($(BREW),)
	@echo "Installing dependencies using $(BREW)"
	@brew install --quiet zmq binaryen lcov
endif
	@echo "Installing grcov using cargo"
	@cargo install grcov
	@echo "Installing mdbook and mdbook-linkcheck using cargo"
	@cargo install mdbook
	@cargo install mdbook-linkcheck
	@echo "installing wasm optimization tools"
	@cargo install wasm-gc wasm-snip

.PHONY: clean_examples
clean_examples:
	@find flowr/examples -name manifest.json | xargs rm -rf
	@find flowr/examples -name \*.svg -not -name test_\* | xargs rm -rf
	@find flowr/examples -name \*.wasm | xargs rm -rf

.PHONY: clean
clean: clean_examples clean-traces
	@echo "clean<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@cargo clean
	@find . -name target -type d | xargs rm -rf
	@find flowr/examples -name "flow_trace.json" -delete 2>/dev/null || true
	@rm -rf $(HOME)/.flow/lib/flowstdlib

.PHONY: build
build:
	@echo "build<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	cargo build
	@target/debug/flowc -d -g -O flowstdlib
	@target/debug/flowc flowr/src/bin/flowrcli
	@target/debug/flowc flowr/src/bin/flowrgui

.PHONY: clippy
clippy:
	@echo "clippy<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	cargo clippy --tests --no-deps --all-features --all-targets -- --warn clippy::pedantic --deny warnings

.PHONY: features
features:
	@echo "features<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@which cargo-build-all-features > /dev/null || cargo install cargo-all-features
	cargo build-all-features

.PHONY: test
test: build check-binary-paths
	@echo "test<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
ifneq ($(CODESIGN),)
	@echo "Code signing tool \"codesign\" detected"
ifneq ($(SELFCERT),)
	@echo "Self-signing certificate called \"self\" found"
	cargo test --no-run
	@find target -name "flow*" -perm +111 -type f | xargs codesign -s self || true
endif
endif
	cargo test
	cargo test --examples

.PHONY: example
example: build
	@echo "example<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
ifdef EXAMPLE
	cargo test -p flowr --example $(EXAMPLE) -- --nocapture
else
	cargo test --examples
endif

.PHONY: tla
tla: build check-binary-paths
	@echo "tla<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@echo "Generating TLA+ specs for flowr examples..."
	@find flowr/examples -name "FlowRuntimeBase.tla" -delete 2>/dev/null || true
	@for dir in flowr/examples/*/; do \
		if [ -f "$$dir/root.toml" ]; then \
			echo "  $$dir"; \
			target/debug/flowc -c --tla -r flowrcli "$$dir" 2>/dev/null || true; \
		fi; \
	done

TLA2TOOLS := $(shell ls "/Applications/TLA+ Toolbox.app/Contents/Eclipse/tla2tools.jar" 2>/dev/null || echo "")
TLC_CMD = java -XX:+UseParallelGC -cp "$(TLA2TOOLS)" tlc2.TLC -workers auto -deadlock

.PHONY: tla-check
tla-check: tla
	@echo "tla-check<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
ifeq ($(TLA2TOOLS),)
	@echo "TLA+ Toolbox not found — skipping TLC verification"
	@echo "Install with: brew install --cask tla+-toolbox"
else
	@echo "Verifying TLA+ specs with TLC model checker..."
	@FAIL=0; \
	for dir in flowr/examples/*/; do \
		name=$$(basename "$$dir"); \
		tla=$$(find "$$dir" -maxdepth 1 -name "*.tla" ! -name "FlowRuntimeBase.tla" 2>/dev/null | head -1); \
		cfg=$$(find "$$dir" -maxdepth 1 -name "*.cfg" 2>/dev/null | head -1); \
		if [ -n "$$tla" ] && [ -n "$$cfg" ]; then \
			printf "  %-30s" "$$name"; \
			if $(TLC_CMD) -metadir "/tmp/tlc-$$name" -config "$$cfg" "$$tla" > /tmp/tlc-$$name.log 2>&1; then \
				states=$$(grep "distinct states" /tmp/tlc-$$name.log | grep -o "[0-9]* distinct" | head -1); \
				echo "OK ($$states states)"; \
			else \
				echo "FAILED"; \
				tail -5 /tmp/tlc-$$name.log; \
				FAIL=1; \
			fi; \
		fi; \
	done; \
	for tla in specs/*.tla; do \
		case "$$tla" in *FlowRuntimeBase*) continue ;; esac; \
		name=$$(basename "$$tla" .tla); \
		cfg="specs/$$name.cfg"; \
		if [ -f "$$cfg" ]; then \
			printf "  %-30s" "specs/$$name"; \
			if $(TLC_CMD) -metadir "/tmp/tlc-specs-$$name" -config "$$cfg" "$$tla" > /tmp/tlc-specs-$$name.log 2>&1; then \
				states=$$(grep "distinct states" /tmp/tlc-specs-$$name.log | grep -o "[0-9]* distinct" | head -1); \
				echo "OK ($$states states)"; \
			else \
				echo "FAILED"; \
				tail -5 /tmp/tlc-specs-$$name.log; \
				FAIL=1; \
			fi; \
		fi; \
	done; \
	if [ $$FAIL -eq 1 ]; then echo "TLA+ verification failed"; exit 1; fi
	@echo "All TLA+ specs verified."
endif

.PHONY: clean-traces
clean-traces:
	@find flowr/examples -name "flow_trace.json" -delete 2>/dev/null || true

.PHONY: check-binary-paths
check-binary-paths:
	@for bin in flowrcli flowc flowrdb; do \
		found=$$(which $$bin 2>/dev/null); \
		expected="$(PWD)/target/debug/$$bin"; \
		if [ "$$found" != "$$expected" ]; then \
			echo "ERROR: $$bin resolves to '$$found' instead of '$$expected'"; \
			echo "Ensure target/debug is first in PATH"; \
			exit 1; \
		fi; \
	done

.PHONY: trace
trace: check-binary-paths clean-traces
	@echo "trace<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	cargo build -p flowr --features trace --bin flowrcli --bin flowr-tla-check
	FLOW_TRACE=flow_trace.json cargo test --examples
	@echo "Trace files:"
	@find flowr/examples -name "flow_trace.json"

.PHONY: tla-trace-check
tla-trace-check: trace
	@echo "tla-trace-check<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
ifeq ($(TLA2TOOLS),)
	@echo "TLA+ Toolbox not found — skipping TLC trace verification"
	@echo "Install with: brew install --cask tla+-toolbox"
else
	@TRACE_DIR=$$(mktemp -d); \
	FAIL=0; COUNT=0; \
	for trace in $$(find flowr/examples -name "flow_trace.json"); do \
		name=$$(basename $$(dirname "$$trace")); \
		printf "  %-30s" "$$name"; \
		out="$$TRACE_DIR/$$name"; mkdir -p "$$out"; \
		cp specs/FlowRuntimeBase.tla "$$out/"; \
		target/debug/flowr-tla-check "$$trace" "$$out" 2>/dev/null || { echo "CONVERT FAILED"; FAIL=1; continue; }; \
		if $(TLC_CMD) -metadir "$$TRACE_DIR/tlc-$$name" -config "$$out/TraceCheck.cfg" "$$out/TraceCheck.tla" > "$$out/tlc.log" 2>&1; then \
			echo "OK"; COUNT=$$((COUNT+1)); \
		else \
			echo "TLC FAILED"; tail -5 "$$out/tlc.log"; FAIL=1; \
		fi; \
		rm -f "$$trace"; \
	done; \
	rm -rf "$$TRACE_DIR"; \
	echo "$$COUNT examples verified against TLA+ spec."; \
	if [ $$FAIL -eq 1 ]; then exit 1; fi
endif

.PHONY: coverage
coverage: clean-start
	@echo "coverage<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo build
	@target/debug/flowc -d -g -O flowstdlib
	@target/debug/flowc flowr/src/bin/flowrcli
	@target/debug/flowc flowr/src/bin/flowrgui
ifeq ($(CODESIGN),)
	find target -perm +111 -type f | xargs codesign -fs self
endif
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo test
	@RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE="flow-%p-%m.profraw" cargo test --examples
	@echo "Gathering coverage information"
	@grcov . --binary-path target/debug/ -s . -t lcov --branch --ignore-not-existing --ignore "/*" -o coverage.info
	@lcov --remove coverage.info '/Applications/*' 'target/debug/build/**' 'target/release/build/**' '/usr*' '**/errors.rs' '**/build.rs' 'flowr/examples/**' '*tests/*' -o coverage.info
	@find . -name "*.profraw" | xargs rm -f
	@echo "Generating coverage report"
	@genhtml -o target/coverage --quiet coverage.info
	@echo "View coverage report using 'open target/coverage/index.html'"

.PHONY: book
book: build-book copy-svgs trim-book

.PHONY: build-book
build-book:
	@echo "build-book<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@mdbook build

.PHONE: copy-svgs
copy-svgs:
	@echo "copy-svgs<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
	@for i in $(shell find flowr/examples -name '*.svg' | grep -v test_ ); do \
      mkdir -p target/html/$$(dirname $$i); \
      cp $$i target/html/$$i; \
    done
	@for i in $(shell cd $$HOME/.flow/lib/flowstdlib && find . -name '*.svg' | grep -v test_ ); do \
      mkdir -p target/html/flowstdlib/src/$$(dirname $$i); \
      cp $$HOME/.flow/lib/flowstdlib/$$i target/html/flowstdlib/src/$$i; \
    done

.PHONY: trim-book
trim-book:
	@echo "trim-book<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
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
	@find target/html -name test.stdout | xargs rm -rf {}
	@find target/html -name test.file | xargs rm -rf {}
	@find target/html -name \*.rs | xargs rm -rf {}
	@find target/html -name \*.dump | xargs rm -rf {}
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
	@rm -rf target/html/Makefile
	@rm -rf target/html/.nojekyll
	@rm -rf target/html/coverage.info
	@rm -rf target/html/flowr/examples/mandlebrot/project
	@find target/html -depth -type d -empty -delete

.PHONY: release
release:
	cargo release --workspace --execute minor