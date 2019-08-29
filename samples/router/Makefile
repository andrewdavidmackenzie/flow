NAME=read_section

wasm: src/$(NAME).wasm

src/$(NAME).wasm: target/wasm32-unknown-unknown/release/$(NAME).wasm
	@cp $< $@

target/wasm32-unknown-unknown/release/$(NAME).wasm: src/$(NAME).rs Cargo.toml
	@cargo build --quiet --release --lib --target=wasm32-unknown-unknown
	@wasm-gc $@