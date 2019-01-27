wasm:
#	@cargo build --target=wasm32-unknown-unknown
#	@cp target/wasm32-unknown-unknown/debug/*.wasm src/

measure:
	cargo +nightly build --release
	time target/release/mandlebrot mandel.png 4000x3000 -1.20,0.35 -1,0.20
	diff mandel.png gold_masters/mandel_4000x3000.png
	cargo +nightly bench