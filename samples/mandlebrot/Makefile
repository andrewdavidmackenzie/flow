all: functions-native functions-wasm app-native

functions-native:
	cd pixel_to_point && cp flow.toml Cargo.toml && cargo test
	cd render_pixel && cp flow.toml Cargo.toml && cargo test

functions-wasm:
	cd pixel_to_point && cp flow.toml Cargo.toml && wasm-pack test --node
	cd render_pixel && cp flow.toml Cargo.toml && wasm-pack test --node

app-native:
	cargo +nightly build --release
	time target/release/mandlebrot mandel_40x30.png 40x30 -1.20,0.35 -1,0.20

bench:
	cargo +nightly bench
