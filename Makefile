EMCC := $(shell command -v emcc -v 2> /dev/null)
RUSTUP := $(shell command -v rustup 2> /dev/null)

all: test package

test: test-lib test-cli test-app

test-lib:
	@echo ""
	@echo "------- Started  testing lib ----------------"
	@cargo test --manifest-path lib/Cargo.toml
	@echo "------- Finished testing lib ----------------"

test-cli:
	@echo ""
	@echo "------- Started  testing cli ----------------"
	@cargo test --manifest-path cli/Cargo.toml
	@echo "------- Finished testing cli ----------------"

test-app:
	@echo ""
	@echo "------- Started  testing app ----------------"
	@cargo test --manifest-path ui/Cargo.toml
	@echo "------- Finished testing app ----------------"

package: package-app package-cli

package-cli: cli
	@echo ""
	@echo "------- Started  packaging cli ----------------"
	@echo "------- Finished packaging cli ----------------" # No specific packing steps after build ATM

cli:
	@cargo build --manifest-path cli/Cargo.toml --bin flow

package-app:
	@echo ""
	@echo "------- Started  packaging app ----------------"
	@cd ui && make package
	@echo "------- Finished packaging app ----------------"

run-cli:
	@cargo run --manifest-path cli/Cargo.toml

run-app:
	@cd ui && make run-app

clean:
	rm -rf cli/target
	rm -rf cli/log
	rm -rf lib/target
	rm -rf ui/target
	cd ui && make clean

dependencies.png: dependencies.dot
	@dot -T png -o $@ $^
	@open $@

dependencies.dot: Makefile
	@$(MAKE) -Bnd | make2graph > $@