DOT := $(shell command -v dot 2> /dev/null)
KCOV := $(shell command -v kcov 2> /dev/null)
APTGET := $(shell command -v apt-get 2> /dev/null)
YUM := $(shell command -v yum 2> /dev/null)
STIME = @mkdir -p target;date '+%s' > target/.$@time ; echo ------- Target \'$@\' starting
ETIME = @read st < target/.$@time ; st=$$((`date '+%s'`-$$st)) ; echo ------- Target \'$@\' done in $$st seconds
FLOWSTDLIB_FILES = $(shell find flowstdlib -type f | grep -v manifest.json)
UNAME := $(shell uname)

all:
	$(STIME)
	@PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:/usr/local/opt/lib/pkgconfig:/usr/local/Cellar/glib/2.62.3/lib/pkgconfig:/usr/lib64/pkgconfig"
	@$(MAKE) -s workspace test docs
	$(ETIME)

online := false

ifeq ($(online),true)
features := --features "online_tests"
else
features :=
endif

########## Configure Dependencies ############
config:
	$(STIME)
	@$(MAKE) -s common-config
	@echo "	Detected OS=$(UNAME)"
ifeq ($(UNAME), Linux)
	@$(MAKE) -s config-linux
endif
ifeq ($(UNAME), Darwin)
	@$(MAKE) -s config-darwin
endif
	$(ETIME)

common-config:
	$(STIME)
	@echo "	Installing wasm32 target and clippy command using rustup"
	@export PATH="$$PATH:~/.cargo/bin"
	@rustup --quiet target add wasm32-unknown-unknown
	@rustup --quiet component add clippy
# cargo install wasm-gc || true
# install mdbook and it's linkcheck linter for generating the guide
	@echo "	Installing mdbook and mdbook-linkcheck using cargo"
	@cargo install mdbook
	@cargo install mdbook-linkcheck
	$(ETIME)

config-darwin:
	$(STIME)
	@echo "	Installing macos specific dependencies using brew"
	@brew install gtk+3 glib cairo atk cmake graphviz
	$(ETIME)

config-linux:
	$(STIME)
ifneq ($(YUM),)
	@echo "	Installing linux specific dependencies using $(YUM)"
	@sudo yum --color=auto --quiet install curl-devel elfutils-libelf-devel elfutils-devel openssl-devel binutils-devel
	@sudo yum --color=auto --quiet install graphviz gtk3-devel || true
else ifneq ($(APTGET),)
	@echo "	Installing linux specific dependencies using $(APTGET)"
	@sudo apt-get -y install libcurl4-openssl-dev libelf-dev libdw-dev libssl-dev binutils-dev
	@sudo apt-get -y install graphviz libgtk-3-dev || true
else
	@echo "	Neither apt-get nor yum detected for installing linux specific dependencies"
endif
	$(ETIME)

################### Doc ####################
.PHONY: docs
docs:
	$(STIME)
	@$(MAKE) dot-graphs build-guide code-docs trim-docs
	$(ETIME)

build-guide:
	@RUST_LOG=info time mdbook build

trim-docs:
	$(STIME)
	@find target/html -name target -type d | xargs rm -rf {}
	@find target/html -name .idea | xargs rm -rf {}
	@find target/html -name \*.iml | xargs rm -rf {}
	@find target/html -name .git | xargs rm -rf {}
	@find target/html -name .sh | xargs rm -rf {}
	@find target/html -name assets | xargs rm -rf {}
	@find target/html -name Cargo.toml | xargs rm -rf {}
	@find target/html -name manifest.json | xargs rm -rf {}
	@find target/html -name test.err | xargs rm -rf {}
	@find target/html -name \*.rs | xargs rm -rf {}
	@find target/html -name pkg | xargs rm -rf {}
	@find target/html -name \*.dump | xargs rm -rf {}
	@find target/html -name \*.dot | xargs rm -rf {}
	@find target/html -name \*.wasm | xargs rm -rf {}
	@find target/html -name \*.lock  | xargs rm -rf {}
	@cd target/html;rm -f Makefile .crates.toml .DS_Store .gitignore .mdbookignore .travis.yml coverage.sh
	@cd target/html;rm -rf bin
	@rm -rf target/html/flowc/tests/test-flows
	@rm -rf target/html/flowc/tests/test-libs
	@rm -rf target/html/code/debug
	@find target/html -depth -type d -empty -delete
	$(ETIME)

code-docs:
	$(STIME)
	@cargo doc --workspace --quiet --all-features --no-deps --target-dir=target/html/code
	$(ETIME)

.PHONY: pages
pages: docs deploy-pages

.PHONY: deploy-pages
deploy-pages:
	$(STIME)
	@echo "====> deploying guide to github"
	git worktree prune
	@rm -rf /tmp/guide
	git worktree add /tmp/guide gh-pages
	rm -rf /tmp/guide/*
	cp -rp target/html/* /tmp/guide/
	cd /tmp/guide && \
		git add -A && \
		git commit -m "deployed on $(shell date) by ${USER}" && \
		git push --force origin gh-pages
	$(ETIME)

#################### Build ####################
workspace: clippy
	$(STIME)
	@cargo build
	$(ETIME)

clippy:
	$(STIME)
	@cargo clippy -- -D warnings
	$(ETIME)

flowrunner:
	$(STIME)
	@cargo build -p flowr
	$(ETIME)

#################### Tests ####################
test:
	$(STIME)
	@$(MAKE) test-workspace samples book-test
	$(ETIME)

test-workspace:
	$(STIME)
	@cargo test $(features)
	$(ETIME)

book-test:
	$(STIME)
	mdbook test
	$(ETIME)

################### Coverage ####################
.PHONY: coverage
coverage: build-kcov measure upload_coverage

upload_coverage:
	@echo "Uploading coverage to https://codecov.io....."
	@curl -s https://codecov.io/bash | bash

#flow and provider .d
measure:
	@echo "Measuring coverage using 'kcov'"
	@for file in `find target/debug -name "flow*.d"`; do ls -l $$file; mkdir -p "target/cov/$(basename $$file)"; kcov --exclude-pattern=/.cargo,/usr/lib "target/cov/$(basename $$file)" "$$file"; done
	#@for file in `find target/debug -name "provider*.d"`; do mkdir -p "target/cov/$(basename $$file)"; kcov --exclude-pattern=/.cargo,/usr/lib "target/cov/$(basename $$file)" "$$file"; done

build-kcov:
ifeq ($(KCOV),)
	@echo "'kcov' is not installed. Building and installing it"
	@echo "Downloading kcov source code"
	@cd target && rm -f target/master.tar.gz && wget -q https://github.com/SimonKagstrom/kcov/archive/master.tar.gz
	@echo "Untarring downloaded kcov tarball"
	@cd target && rm -rf kcov-master && tar xzf master.tar.gz
	@echo "Building kcov from source"
ifeq ($(UNAME), Linux)
	@cd target/kcov-master && rm -rf build && mkdir build && cd build && cmake .. && make && sudo make install
endif
ifeq ($(UNAME), Darwin)
	@echo "Installing 'openssl' and 'binutils' with brew"
	@brew install binutils 2>/dev/null; true
	@# Remove python 2 to avoid dependency issue on osx
	@# https://askubuntu.com/questions/981663/python2-7-broken-by-weakref-import-error-please-help
	@brew remove python@2 --ignore-dependencies 2>/dev/null; true
	@echo "Installing required python packages: 'six'"
	@pip3 -q install six 2>/dev/null
	@echo "Linking openssl to a place where the compiler looks for it"
	@ln -s /usr/local/opt/openssl/include/openssl /usr/local/include 2>/dev/null; true
	@ln -s /usr/local/Cellar/openssl@1.1/1.1.1g/include/openssl /usr/bin/openssl 2>/dev/null; true
	@ln -s /usr/local/opt/openssl/lib/libssl.1.1.1.dylib /usr/local/lib/ 2>/dev/null; true
	#ln -s /usr/local/Cellar/openssl@1.1/1.1.1g/lib/libssl.dylib /usr/local/lib/
	#ln -s /usr/local/Cellar/openssl@1.1/1.1.1g/lib/libcrypto.1.1.dylib /usr/local/lib/
	@cd target/kcov-master && mkdir build && cd build && cmake -G Xcode .. &&  xcodebuild -configuration Release
	#@cd target/kcov-master && mkdir build && cd build && cmake .. && make && xcodebuild -configuration Release
	@sudo mv src/Release/kcov /usr/local/bin/kcov
endif
	@rm -rf kcov-master
	@rm -f master.tar.gz*
else
	@echo "'kcov' found, skipping build of it"
endif

#################### FLOW LIBRARIES ####################
flowstdlib: flowstdlib/manifest.json

flowstdlib/manifest.json: $(FLOWSTDLIB_FILES)
	@mkdir -p target;date '+%s' > target/.flowstdlibtime ; echo \\n------- Target \'$@\' starting
	@cargo run -p flowc -- -v info -l -g -d flowstdlib
	@read st < target/.flowstdlibtime ; st=$$((`date '+%s'`-$$st)) ; echo ------- Target \'$@\' done in $$st seconds

#################### Raspberry Pi ####################
pi:
	@echo "Building flowc for pi in $(PWD)"
# https://hub.docker.com/r/dlecan/rust-crosscompiler-arm
	docker run -it --rm -v $(PWD):/source -v ~/.cargo/git:/root/.cargo/git -v ~/.cargo/registry:/root/.cargo/registry dlecan/rust-crosscompiler-arm:stable
# In case of permissions problems for cargo cache on local machine:
# sudo chown -R `stat -c %u:%g $HOME` $(pwd) ~/.cargo

copy:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/release/flowc andrew@zero-w:
	scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no target/arm-unknown-linux-gnueabihf/release/flowr andrew@zero-w:

#################### SAMPLES ####################
# Find all sub-directories under 'samples' and create a list of paths like 'sample/{directory}/test.output' to use for
# make paths - to compile all samples found in there. Avoid files using the filter.
sample_flows := $(patsubst samples/%,samples/%test.output,$(filter %/, $(wildcard samples/*/)))

# This target must be below sample-flows in the Makefile
samples: flowrunner flowstdlib/manifest.json
	$(STIME)
	@cd samples; $(MAKE) clean
	@$(MAKE) $(sample_flows)
	$(ETIME)

samples/%: samples/%/test.err
	$(MAKE) $(@D)/test.output

samples/%/test.output: samples/%/test.input samples/%/test.arguments
	@printf "\tSample '$(@D)'"
	@RUST_BACKTRACE=1 cat $< | cargo run --quiet -p flowc -- -g -d $(@D) -- `cat $(@D)/test.arguments` 2> $(@D)/test.err > $@
	@diff $@ $(@D)/expected.output || (ret=$$?; cp $@ $(@D)/failed.output && rm -f $@ && exit $$ret)
	@if [ -s $(@D)/test.err ]; then (printf " has error output in $(@D)/test.err\n"; exit -1); else printf " has no errors\n"; fi;
	@rm $@ #remove test.output after successful diff so that dependency will cause it to run again next time
# leave test.err for inspection in case of failure

################# ONLINE SAMPLES ################
online-samples:
	$(STIME)
	@echo "Hello" | cargo run --p flowc -- https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/samples/hello-world-simple/context.toml
	$(ETIME)

################# Packaging ################
#### Due to dependencies between packages, they need to be published in a given order. Basically this is a DAG and
#### you need to publish from the leaves at the bottom, upwards. I have separated into layers as there are some
#### groups of packages (in same layer) that have same dependencies but are indpendant and they could be published
#### in parallel. But they both need to be published before the next layer up.
#### Level 0 - the root
publish: flowc_publish flowide_publish

#### Level 1 - flowc and flowide - no dependency between them
flowc_publish: flowr_publish flowclib_publish flowrlib_publish provider_publish
	cargo publish --manifest-path=flowc/Cargo.toml

flowide_publish: flowclib_publish flowrlib_publish provider_publish flow_impl_publish flowstdlib_publish
	cargo publish --manifest-path=flowide/Cargo.toml

#### Level 2 - flowr
flowr_publish: provider_publish flow_impl_publish flowstdlib_publish
	cargo publish --manifest-path=flowr/Cargo.toml

#### Level 3 - provider
provider_publish: flowrlib_publish
	cargo publish --manifest-path=provider/Cargo.toml

#### Level 4 - flowstdlib and flowclib
flowstdlib_publish: flow_impl_publish flow_impl_derive_publish flowrlib_publish
	cargo publish --manifest-path=flowstdlib/Cargo.toml

flowclib_publish: flowrlib_publish
	cargo publish --manifest-path=flowclib/Cargo.toml

#### Level 5 - flowruntime
flowruntime_publish: flow_impl_publish flowrlib_publish

#### Level 6 - flowrlib
flowrlib_publish: flow_impl_publish
	cargo publish --manifest-path=flowrlib/Cargo.toml

#### Level 7 - flow_impl_publish flow_impl_derive_publish
flow_impl_publish:
	cargo publish --manifest-path=flow_impl/Cargo.toml

flow_impl_derive_publish:
	cargo publish --manifest-path=flow_impl_derive/Cargo.toml

################# Clean ################
clean:
	$(STIME)
	@$(MAKE) clean-dumps clean-guide
	@cd flowstdlib; $(MAKE) clean
	@cd samples; $(MAKE) clean
	@cargo clean
	$(STIME)

clean-dumps:
	$(STIME)
	@find . -name \*.dump -type f -exec rm -rf {} + ; true
	@find . -name \*.dot -type f -exec rm -rf {} + ; true
	@find . -name \*.dot.svg -type f -exec rm -rf {} + ; true
	@echo "\tAll .dump, .dot and .dot.svg files removed"
	$(ETIME)

clean-guide:
	$(STIME)
	@rm -rf target/html
	$(ETIME)

################# Dot Graphs ################
dot-graphs:
ifeq ($(DOT),)
	@echo "\t'dot' not available, skipping 'dot-graphs'. Install 'graphviz' to use."
else
	@find . -name \*.dot -type f -exec dot -Tsvg -O {} \;
	@echo "\tGenerated .svg files for all dot graphs found"
endif