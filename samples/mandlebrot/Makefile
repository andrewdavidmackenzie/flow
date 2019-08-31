NAME=parse_pair
TMPDIR := $(shell mktemp -d)

wasm: src/$(NAME)/$(NAME).wasm

src/$(NAME)/$(NAME).wasm: $(TMPDIR)/wasm32-unknown-unknown/release/$(NAME).wasm
	@cp $< $@

$(TMPDIR)/wasm32-unknown-unknown/release/$(NAME).wasm: src/$(NAME)/$(NAME).rs src/$(NAME)/Cargo.toml
	@cargo build --quiet --manifest-path=src/$(NAME)/Cargo.toml --release --target-dir=$(TMPDIR) --lib --target=wasm32-unknown-unknown
	@wasm-gc $@
	@echo "Generated wasm implementation for function '$(NAME)'"