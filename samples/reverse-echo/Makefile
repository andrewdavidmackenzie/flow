NAME=reverse

wasm: src/$(NAME).wasm

src/$(NAME).wasm: target/wasm32-unknown-unknown/debug/$(NAME).wasm
	@cp $< $@

target/wasm32-unknown-unknown/debug/$(NAME).wasm: src/$(NAME).rs Cargo.toml
	@cargo build --quiet --lib --target=wasm32-unknown-unknown