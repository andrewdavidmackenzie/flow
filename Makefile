EMCC := $(shell command -v emcc -v 2> /dev/null)
RUSTUP := $(shell command -v rustup 2> /dev/null)

all: test package

test: test-lib test-flowc test-app

test-lib:
	@echo ""
	@echo "------- Started  testing lib ----------------"
	@cargo test --manifest-path lib/Cargo.toml
	@echo "------- Finished testing lib ----------------"

test-flowc:
	@echo ""
	@echo "------- Started  testing flowc ----------------"
	@cargo test --manifest-path flowc/Cargo.toml
	@echo "------- Finished testing flowc ----------------"

test-app:
	@echo ""
	@echo "------- Started  testing app ----------------"
	@cargo test --manifest-path ui/Cargo.toml
	@echo "------- Finished testing app ----------------"

package: package-app package-flowc

package-flowc: flowc
	@echo ""
	@echo "------- Started  packaging flowc ----------------"
	@echo "------- Finished packaging flowc ----------------" # No specific packing steps after build ATM

flowc:
	@cargo build --manifest-path flowc/Cargo.toml --bin flow

package-app:
	@echo ""
	@echo "------- Started  packaging app ----------------"
	@cd ui && make package
	@echo "------- Finished packaging app ----------------"

run-flowc:
	@cargo run --manifest-path flowc/Cargo.toml

run-app:
	@cd ui && make run-app

clean:
	rm -rf flowc/target
	rm -rf flowc/log
	rm -rf lib/target
	rm -rf ui/target
	cd ui && make clean

dependencies.png: dependencies.dot
	@dot -T png -o $@ $^
	@open $@

dependencies.dot: Makefile
	@$(MAKE) -Bnd | make2graph > $@