NAME=parse_pair
TMPDIR := $(shell mktemp -d)

wasm: src/$(NAME)/$(NAME).wasm

src/$(NAME)/$(NAME).wasm: $(TMPDIR)/wasm32-unknown-unknown/release/$(NAME).wasm
	@cp $< $@

$(TMPDIR)/wasm32-unknown-unknown/release/$(NAME).wasm: src/$(NAME)/$(NAME).rs src/$(NAME)/Cargo.toml
	@cd src/$(NAME);cargo build --quiet --release --target-dir=$(TMPDIR) --lib --target=wasm32-unknown-unknown
	@wasm-gc $@
	@echo "Generated wasm implementation for function '$(NAME)'"