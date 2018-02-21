EMCC := $(shell command -v emcc -v 2> /dev/null)
RUSTUP := $(shell command -v rustup 2> /dev/null)

all: test package doc

test: local-tests online-tests test-gtk

online := true

ifeq ($(online),true)
features := --features "online_tests"
else
features :=
endif

doc:
	cargo doc

# In Travis don't try to test gtk as needs many extra installs
travis: local-tests online-tests

local-tests: test-flowclib test-flowrlib test-flowstdlib test-flowc test-electron test-samples

online-tests: test-hello-simple-online

#TODO map the cargo cache as a volume to avoid re-downloading and compiling every time.
pi:
	@echo "Building flowc for pi in $(PWD)"
	docker run -e "PKG_CONFIG_ALLOW_CROSS=1" --volume $(PWD):/home/cross/project rust-nightly-pi-cross build
	@./target/debug/flowc samples/fibonacci
	docker run -e "PKG_CONFIG_ALLOW_CROSS=1" --volume $(PWD):/home/cross/project rust-nightly-pi-cross build --manifest-path samples/fibonacci/Cargo.toml

copy:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/debug/flowc pi@raspberrypi.local:

#################### Libraries ####################
test-flowclib:
	@echo ""
	@echo "------- Started  testing flowclib -------------"
	@cargo test --manifest-path flowclib/Cargo.toml $(features)
	@echo "------- Finished testing flowclib -------------"

test-flowrlib:
	@echo ""
	@echo "------- Started  testing flowrlib -------------"
	@cargo test --manifest-path flowrlib/Cargo.toml
	@echo "------- Finished testing flowrlib -------------"

test-flowstdlib:
	@echo ""
	@echo "------- Started  testing flowstdlib -------------"
	@cargo test --manifest-path flowstdlib/Cargo.toml
	@echo "------- Finished testing flowstdlib -------------"

################### CLI BINARY ##################
test-flowc: ./target/debug/flowc
	@echo ""
	@echo "------- Started  testing flowc ----------------"
	@cargo test --manifest-path flowc/Cargo.toml
	@echo "------- Finished testing flowc ----------------"

./target/debug/flowc: flowc
	@cargo build --manifest-path flowc/Cargo.toml

#################### SAMPLES ####################
sample_flows := $(patsubst samples/%,samples/%/rust/target,$(wildcard samples/*))

test-samples: $(sample_flows)

samples/%/rust/target : samples/%/context.toml
	@echo "------- Compiling and Running flow: $< ----"
	./target/debug/flowc $<

clean-samples:
	find samples -name rust -delete

################# ONLINE SAMPLES ################
test-hello-simple-online: ./target/debug/flowc
	@echo ""
	@echo "------- Started testing generation of hello-world-simple-online ----"
	./target/debug/flowc https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml
	@echo "------- Finished testing generation of hello-world-simple-online ----"

################## ELECTRON UI ##################
test-electron:
	@echo ""
	@echo "------- Started  testing electron -------------"
	@cargo test --manifest-path electron/Cargo.toml
	@echo "------- Finished testing electron -------------"

#################### GTK UI ####################
test-gtk:
	@echo ""
	@echo "------- Started  testing gtk -------------"
	@cargo test --manifest-path gtk/Cargo.toml
	@echo "------- Finished testing gtk -------------"

package: package-electron package-flowc

package-flowc:
	@echo ""
	@echo "------- Started  packaging flowc --------------"
	@cargo package --manifest-path flowc/Cargo.toml
	@echo "------- Finished packaging flowc --------------"

package-electron:
	@echo ""
	@echo "------- Started  packaging electron -----------"
	@cd electron && make package
	@echo "------- Finished packaging electron -----------"

run-flowc:
	@cargo run --manifest-path flowc/Cargo.toml

run-electron:
	@cd electron && make run-electron

clean: clean-samples
	cargo clean
	cd electron && make clean

dependencies.png: dependencies.dot
	@dot -T png -o $@ $^
	@open $@

dependencies.dot: Makefile
	@$(MAKE) -Bnd | make2graph > $@
