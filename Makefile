EMCC := $(shell command -v emcc -v 2> /dev/null)
RUSTUP := $(shell command -v rustup 2> /dev/null)

all: test package

test: test-flowclib test-flowc test-electron

test-flowclib:
	@echo ""
	@echo "------- Started  testing flowclib ----------------"
	@cargo test --manifest-path flowclib/Cargo.toml
	@echo "------- Finished testing flowclib ----------------"

test-flowc:
	@echo ""
	@echo "------- Started  testing flowc ----------------"
	@cargo test --manifest-path flowc/Cargo.toml
	@echo "------- Finished testing flowc ----------------"

test-electron:
	@echo ""
	@echo "------- Started  testing electron ----------------"
	@cargo test --manifest-path electron/Cargo.toml
	@echo "------- Finished testing electron ----------------"

package: package-electron package-flowc

package-flowc: flowc
	@echo ""
	@echo "------- Started  packaging flowc ----------------"
	@echo "------- Finished packaging flowc ----------------" # No specific packing steps after build ATM

flowc:
	@cargo build --manifest-path flowc/Cargo.toml --bin flow

package-electron:
	@echo ""
	@echo "------- Started  packaging electron ----------------"
	@cd electron && make package
	@echo "------- Finished packaging electron ----------------"

run-flowc:
	@cargo run --manifest-path flowc/Cargo.toml

run-electron:
	@cd electron && make run-electron

clean:
	rm -rf flowc/target
	rm -rf flowc/log
	rm -rf flowclib/target
	rm -rf electron/target
	cd electron && make clean

dependencies.png: dependencies.dot
	@dot -T png -o $@ $^
	@open $@

dependencies.dot: Makefile
	@$(MAKE) -Bnd | make2graph > $@