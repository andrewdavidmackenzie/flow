EMCC := $(shell command -v emcc -v 2> /dev/null)
RUSTUP := $(shell command -v rustup 2> /dev/null)

all: test package doc

online := true

ifeq ($(online),true)
features := --features "online_tests"
else
features :=
endif

doc:
	cargo doc

test: travis online-tests

# In Travis don't try to test gtk as needs many extra installs
travis: local-tests

local-tests: test-flow test-samples

online-tests: test-hello-simple-online

#TODO map the cargo cache as a volume to avoid re-downloading and compiling every time.
pi:
	@echo "Building flowc for pi in $(PWD)"
	docker run -e "PKG_CONFIG_ALLOW_CROSS=1" --volume $(PWD):/home/cross/project rust-nightly-pi-cross build
	@./target/debug/flowc samples/fibonacci
	docker run -e "PKG_CONFIG_ALLOW_CROSS=1" --volume $(PWD):/home/cross/project rust-nightly-pi-cross build --manifest-path samples/fibonacci/Cargo.toml

copy:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/debug/flowc pi@raspberrypi.local:

#################### Flow ####################
test-flow:
	@echo ""
	@echo "------- Started  testing flow -------------"
	@cargo test $(features)
	@echo "------- Finished testing flow -------------"

compiler:
	@cargo build --manifest-path=flowc/Cargo.toml

#################### SAMPLES ####################
sample_flows := $(patsubst samples/%,samples/%/test_output.txt,$(wildcard samples/*))

test-samples: $(sample_flows)

samples/%/test_output.txt : samples/%/test_input.txt compiler
	@echo "\n------- Compiling and Running sample $(@D) ----"
	@cat $< | ./target/debug/flowc $(@D) > $@
	diff $@ $(@D)/expected_output.txt
	@rm $@

################# ONLINE SAMPLES ################
test-hello-simple-online: ./target/debug/flowc
	@echo ""
	@echo "------- Started testing generation of hello-world-simple-online ----"
	@echo "Hello" | ./target/debug/flowc https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml
	@echo "------- Finished testing generation of hello-world-simple-online ----"

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

run-electron:
	@cd electron && make run-electron

clean:
	cargo clean
	@find samples -name rust -type d -exec rm -rf {} + ; true
	@find samples -name test_output.txt -exec rm -rf {} + ; true
	cd electron && make clean