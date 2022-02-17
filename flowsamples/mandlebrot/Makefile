all: functions-native functions-wasm app-native

functions-native:
	cd pixel_to_point && cp flow.toml Cargo.toml && cargo test
	cd escapes && cargo test

functions-wasm:
	cd pixel_to_point && cp flow.toml Cargo.toml && wasm-pack test --node
	cd escapes wasm-pack test --node

app-native:
	cargo +nightly build --release
	time target/release/mandlebrot mandle_20x15.png 20x15 -1.20,0.35 -1,0.20

bench:
	cargo +nightly bench
