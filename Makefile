RUSTUP := $(shell command -v rustup 2> /dev/null)

all: build test doc
	@echo ""
	@echo "**************************************"
	@echo "************* Done all: **************"
	@echo "**************************************"

online := false

ifeq ($(online),true)
features := --features "online_tests"
else
features :=
endif

########## Configure Dependencies ############
config:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-bindgen-cli || true
	cargo install mdbook || true
	cargo install mdbook-linkcheck || true

#################### Docs ####################
doc: guide
	@echo ""
	@echo "------- Started building docs with cargo -------------"
	cargo doc
	@echo "------- Ended   building docs with cargo -------------"

################### Guide ####################
guide: copy-md-files
	@echo ""
	@echo "------- Started building book from Markdown into 'guide/book/html' -------------"
	@mdbook build guide
	@echo "------- Done    building book from Markdown into 'guide/book/html' -------------"

## Copy .md files (with same directory sturtcure) from samples and lib directories under guide 'src' directory
copy-md-files:
	@echo ""
	@echo "------- Started copying Markdown files from 'samples' to 'guide/src' -------------"
	@find samples -type f -name \*.md -exec dirname '{}' ';' | xargs printf 'guide/src/%s\n' | xargs mkdir -p
	@find samples -type f -name \*.md -exec cp '{}' guide/src/'{}' ';'

	@echo "------- Started copying Markdown files from 'flowstdlib' to 'guide/src' -------------"
	@find flowstdlib -type f -name \*.md -exec dirname '{}' ';' | xargs printf 'guide/src/%s\n' | xargs mkdir -p
	@find flowstdlib -type f -name \*.md -exec cp '{}' guide/src/'{}' ';'

	@echo "------- Started copying Markdown files from 'flowrlib' to 'guide/src' -------------"
	@find flowrlib -type f -name \*.md -exec dirname '{}' ';' | xargs printf 'guide/src/%s\n' | xargs mkdir -p
	@find flowrlib -type f -name \*.md -exec cp '{}' guide/src/'{}' ';'

	@echo "------- Done    copying Markdown files from 'samples' and 'flowstdlib' to 'guide/src' -------------"

#################### Build ####################
build: flowc web flowclib flowstdlib flowrlib flowclilib
	@echo "------- Done 'build:' target -------------"

./target/debug/flowc: flowc
	cargo build

flowc: flowclib
	@echo ""
	@echo "------- Starting build of 'flow' workspace project -------------"
	cargo build
	@echo "------- Done     build of 'flow' workspace project -------------"

web: flowrlib flowstdlib
	cd web && make build

flowclib:
	cd flowclib && make build

flowstdlib: flowrlib
	cd flowstdlib && make build

flowrlib:
	cd flowrlib && make build

flowclilib:
	cd flowclilib && make build

################## Travis CI ##################
travis: clean test guide

#################### Tests ####################
test: test-flowc test-web test-flowclib test-flowstdlib test-flowrlib test-flowclilib local-samples
# TYODO add online-samples
	@echo ""
	@echo "------- Done    test: -------------"

test-flowc:
	@echo ""
	@echo "------- Starting build of 'flow' workspace project -------------"
	cargo test $(features)
	@echo "------- Done     build of 'flow' workspace project -------------"

test-web:
	cd web && make test

test-flowclib:
	cd flowclib && make test

test-flowstdlib:
	cd flowstdlib && make test

test-flowrlib:
	cd flowrlib && make test

test-flowclilib:
	cd flowclilib && make test

#################### Raspberry Pi ####################
#TODO map the cargo cache as a volume to avoid re-downloading and compiling every time.
pi:
	@echo "Building flowc for pi in $(PWD)"
	docker run -e "PKG_CONFIG_ALLOW_CROSS=1" --volume $(PWD):/home/cross/project rust-nightly-pi-cross build
	@./target/debug/flowc samples/fibonacci
	docker run -e "PKG_CONFIG_ALLOW_CROSS=1" --volume $(PWD):/home/cross/project rust-nightly-pi-cross build --manifest-path samples/fibonacci/Cargo.toml

copy:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/debug/flowc andrew@raspberrypi.local:

#################### SAMPLES ####################
# Find all sub-directories under 'samples' and create a list of paths like 'sample/{directory}/test_output.txt' to use for
# make paths - to compile all samples found in there. Avoid files using the filter.
sample_flows := $(patsubst samples/%,samples/%test_output.txt,$(filter %/, $(wildcard samples/*/)))

local-samples: ./target/debug/flowc $(sample_flows)  # This target must be below sample-flows in the Makefile
	@echo ""
	@echo "All local samples executed and output as expected"
	@echo "------- Finished 'local-samples:' ----"

clean-samples:
	@find samples -name rust -type d -exec rm -rf {} + ; true
	@find samples -name test_output.txt -exec rm -rf {} + ; true

samples/%/test_output.txt: samples/%/test_input.txt
	@echo "\n------- Compiling and Running '$(@D)' ----"
# remove local file path from output messages with sed to make local failures match travis failures
	@cat $< | ./target/debug/flowc -d $(@D) -- `cat $(@D)/test_arguments.txt` | sed -e 's/\/.*\/flow\///' | grep -v "Finished dev" > $@; true
	@diff $@ $(@D)/expected_output.txt
	@echo "Sample output matches expected output"
	@rm $@ #remove test_output.txt after successful diff so that dependency will cause it to run again next time

################# ONLINE SAMPLES ################
online-samples: test-hello-simple-online

test-hello-simple-online: ./target/debug/flowc
	@echo ""
	@echo "------- Started testing generation of hello-world-simple-online ----"
	@echo "Hello" | cargo run flowc -- https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml
	@echo "------- Finished testing generation of hello-world-simple-online ----"

################# Packaging ################
#package: package-electron package-flowc
package: package-flowc

package-flowc:
	@echo ""
	@echo "------- Started  packaging flowc --------------"
	@cargo package --manifest-path flowc/Cargo.toml
	@echo "------- Finished packaging flowc --------------"

package-electron: web
	@echo ""
	@echo "------- Started  packaging electron -----------"
	@cd electron && make package
	@echo "------- Finished packaging electron -----------"

############## Electron version #############
run-electron:
	@cd electron && make run-electron

################# Clean ################
clean: clean-samples clean-dumps
	cargo clean
	@rm -rf guide/book
	cd electron && make clean
	cd web && make clean
	cd flowclib && make clean
	cd flowstdlib && make clean
	cd flowrlib && make clean
	cd flowclilib && make clean

clean-dumps:
	@find . -name \*.dump -type f -exec rm -rf {} + ; true
	@find . -name \*.dot -type f -exec rm -rf {} + ; true
	@echo "All .dump and .dot files removed"

