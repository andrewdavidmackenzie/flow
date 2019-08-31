NAME=read_section
TMPDIR := $(shell mktemp -d)

wasm: $(NAME)/$(NAME).wasm

$(NAME)/$(NAME).wasm: $(TMPDIR)/wasm32-unknown-unknown/release/$(NAME).wasm
	@cp $< $@

$(TMPDIR)/wasm32-unknown-unknown/release/$(NAME).wasm: $(NAME)/$(NAME).rs $(NAME)/Cargo.toml
	@cd $(NAME);cargo build --quiet --release --target-dir=$(TMPDIR) --lib --target=wasm32-unknown-unknown
	@wasm-gc $@
	@echo "Generated wasm implementation for function '$(NAME)'"