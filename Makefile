EMCC := $(shell command -v emcc -v 2> /dev/null)
RUSTUP := $(shell command -v rustup 2> /dev/null)

all: test package doc

test: local-tests test-gtk

doc:
	cargo doc

travis: local-tests online-tests

local-tests: test-flowclib test-flowrlib test-flowc test-hello-world-simple test-fibonacci test-electron

online-tests: test-hello-simple-online

#TODO map the cargo cache as a volume to avoid re-downloading and compiling every time.
pi:
	@echo "Building flowc for pi in $(PWD)"
	docker run -e "PKG_CONFIG_ALLOW_CROSS=1" --volume $(PWD):/home/cross/project rust-nightly-pi-cross build
	@./target/debug/flowc samples/fibonacci
	docker run -e "PKG_CONFIG_ALLOW_CROSS=1" --volume $(PWD):/home/cross/project rust-nightly-pi-cross build --manifest-path samples/fibonacci/Cargo.toml

copy:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/debug/flowc pi@raspberrypi.local:

test-flowclib:
	@echo ""
	@echo "------- Started  testing flowclib -------------"
	@cargo test --manifest-path flowclib/Cargo.toml
	@echo "------- Finished testing flowclib -------------"

test-flowrlib:
	@echo ""
	@echo "------- Started  testing flowrlib -------------"
	@cargo test --manifest-path flowrlib/Cargo.toml
	@echo "------- Finished testing flowrlib -------------"

./target/debug/flowc:
	@cargo build --manifest-path flowc/Cargo.toml

test-flowc: ./target/debug/flowc
	@echo ""
	@echo "------- Started  testing flowc ----------------"
	@cargo test --manifest-path flowc/Cargo.toml
	@echo "------- Finished testing flowc ----------------"

test-hello-world-simple: ./target/debug/flowc
	@echo ""
	@echo "------- Started testing generation of hello-world-simple ----"
	@rm -rf samples/hello-world-simple/rust
	./target/debug/flowc samples/hello-world-simple
	@cargo run --manifest-path  samples/hello-world-simple/Cargo.toml
	@echo "------- Finished testing generation of hello-world-simple ----"

test-hello-simple-online: ./target/debug/flowc
	@echo ""
	@echo "------- Started testing generation of hello-world-simple-online ----"
	./target/debug/flowc https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml
#	@cargo run --manifest-path  samples/hello-world-simple/Cargo.toml
	@echo "------- Finished testing generation of hello-world-simple-online ----"

# NOTE for now it only builds it, doesn't run it as it crashes with interger overflow
test-fibonacci: ./target/debug/flowc
	@echo ""
	@echo "------- Started testing generation of fibonacci ----"
	@rm -rf samples/fibonacci/src
	./target/debug/flowc samples/fibonacci
	@cargo build --manifest-path  samples/fibonacci/Cargo.toml
	@echo "------- Finished testing generation of fibonacci ----"

test-electron:
	@echo ""
	@echo "------- Started  testing electron -------------"
	@cargo test --manifest-path electron/Cargo.toml
	@echo "------- Finished testing electron -------------"

test-gtk:
	@echo ""
	@echo "------- Started  testing gtk -------------"
	@cargo test --manifest-path gtk/Cargo.toml
	@echo "------- Finished testing gtk -------------"

package: package-electron package-flowc

package-flowc: flowc
	@echo ""
	@echo "------- Started  packaging flowc --------------"
	@echo "------- Finished packaging flowc --------------" # No specific packing steps after build ATM

flowc:
	@cargo build --manifest-path flowc/Cargo.toml --bin flow

package-electron:
	@echo ""
	@echo "------- Started  packaging electron -----------"
	@cd electron && make package
	@echo "------- Finished packaging electron -----------"

run-flowc:
	@cargo run --manifest-path flowc/Cargo.toml

run-electron:
	@cd electron && make run-electron

clean:
	rm -rf target
	rm -rf flowc/target
	rm -rf flowc/log
	rm -rf flowclib/target
	rm -rf flowrlib/target
	rm -rf flowstdlib/target
	rm -rf electron/target
	rm -rf samples/hello-world-simple/rust
	cd electron && make clean

dependencies.png: dependencies.dot
	@dot -T png -o $@ $^
	@open $@

dependencies.dot: Makefile
	@$(MAKE) -Bnd | make2graph > $@
