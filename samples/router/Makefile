NAME=read_section

wasm: $(NAME)/$(NAME).wasm

$(NAME)/$(NAME).wasm: $(NAME)/target/wasm32-unknown-unknown/release/$(NAME).wasm
	@cp $< $@

$(NAME)/target/wasm32-unknown-unknown/release/$(NAME).wasm: $(NAME)/$(NAME).rs read_section/Cargo.toml
	@cd $(NAME);cargo build --quiet --release --lib --target=wasm32-unknown-unknown
	@wasm-gc $@