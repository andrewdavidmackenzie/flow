measure:
	cargo +nightly build --release
	time target/release/mandlebrot mandel_400x300.png 400x300 -1.20,0.35 -1,0.20
	cargo +nightly bench