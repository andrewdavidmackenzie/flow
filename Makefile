RUSTUP := $(shell command -v rustup 2> /dev/null)

all: test doc

online := false

ifeq ($(online),true)
features := --features "online_tests"
else
features :=
endif

config:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-bindgen-cli || true
	cargo install mdbook || true
	cargo install mdbook-linkcheck || true

doc: build-guide
	cargo doc

#################### Guide ####################
build-guide: copy-md-files
	@echo ""
	@echo "------- Building book from Markdown into 'guide/home' -------------"
	@mdbook build guide

## Copy .md files (with same directory sturtcure) from samples and flowstdlib directories under guide 'src' directory
copy-md-files:
	@echo ""
	@echo "------- Copying Markdown files from 'samples' and 'flowstdlib' to 'guide/src' -------------"
	@find samples -type f -name \*.md -exec dirname '{}' ';' | xargs printf 'guide/src/%s\n' | xargs mkdir -p
	@find samples -type f -name \*.md -exec cp '{}' guide/src/'{}' ';'
	@find flowstdlib -type f -name \*.md -exec dirname '{}' ';' | xargs printf 'guide/src/%s\n' | xargs mkdir -p
	@find flowstdlib -type f -name \*.md -exec cp '{}' guide/src/'{}' ';'

#################### Tests ####################
#test: travis online-tests
test: travis

travis: local-tests test-web build-guide

local-tests: test-flow test-samples

online-tests: test-hello-simple-online

#################### Raspberry Pi ####################
#TODO map the cargo cache as a volume to avoid re-downloading and compiling every time.
pi:
	@echo "Building flowc for pi in $(PWD)"
	docker run -e "PKG_CONFIG_ALLOW_CROSS=1" --volume $(PWD):/home/cross/project rust-nightly-pi-cross build
	@./target/debug/flowc samples/fibonacci
	docker run -e "PKG_CONFIG_ALLOW_CROSS=1" --volume $(PWD):/home/cross/project rust-nightly-pi-cross build --manifest-path samples/fibonacci/Cargo.toml

copy:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/debug/flowc andrew@raspberrypi.local:

#################### Flow ####################
test-flow:
	@echo ""
	@echo "------- Started  testing flow -------------"
	@cargo test $(features)
	@echo "------- Finished testing flow -------------"

compiler:
	@echo "------- Started  building flowc -------------"
	@cargo build --manifest-path=flowc/Cargo.toml
	@echo "------- Finished building flowc -------------"

#################### SAMPLES ####################
# Find all sub-directories under 'samples' and create a list of paths like 'sample/{directory}/test_output.txt' to use for
# make paths - to compile all samples found in there. Avoid files using the filter.
sample_flows := $(patsubst samples/%,samples/%test_output.txt,$(filter %/, $(wildcard samples/*/)))

test-samples: $(sample_flows)

samples/%/test_output.txt: samples/%/test_input.txt compiler
	@echo "\n------- Compiling and Running $(@D) ----"
# remove local file path from output messages to make local failures match travis failures
	@cat $< | ./target/debug/flowc -d $(@D) -- `cat $(@D)/test_arguments.txt` | sed -e 's/\/.*\/flow\///' | grep -v "Finished dev" > $@; true
	diff $@ $(@D)/expected_output.txt
	@rm $@

################# ONLINE SAMPLES ################
test-hello-simple-online: ./target/debug/flowc
	@echo ""
	@echo "------- Started testing generation of hello-world-simple-online ----"
	@echo "Hello" | ./target/debug/flowc https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml
	@echo "------- Finished testing generation of hello-world-simple-online ----"

#package: package-electron package-flowc
package: package-flowc

package-flowc:
	@echo ""
	@echo "------- Started  packaging flowc --------------"
	@cargo package --manifest-path flowc/Cargo.toml
	@echo "------- Finished packaging flowc --------------"

package-electron: build-web
	@echo ""
	@echo "------- Started  packaging electron -----------"
	@cd electron && make package
	@echo "------- Finished packaging electron -----------"

test-web:
	@echo ""
	@echo "------- Started test of 'web' -----------------"
	cd web && make test
	@echo "------- Ended   test of 'web' -----------------"

build-web:
	cd web && make

run-electron:
	@cd electron && make run-electron

clean:
	cargo clean
	@find samples -name rust -type d -exec rm -rf {} + ; true
	@find samples -name test_output.txt -exec rm -rf {} + ; true
	cd electron && make clean
