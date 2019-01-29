wasm:
	@cargo build --release --target=wasm32-unknown-unknown
	@cp target/wasm32-unknown-unknown/release/*.wasm src/
# wasm-opt -Oz
# wasm-gc
# wasm-strip
# cargo install wasm-gc
# cargo install wasm-snip
# wasm-gc → wasm-snip → wasm-gc → wasm-opt.