all: build test time bench

build:
	cargo +nightly build --release

test:
	cargo +nightly test

time: build
	time target/release/mandlebrot mandle_20x15.png 20x15 -1.20,0.35 -1,0.20

bench:
	cargo +nightly bench

clean:
	rm -f mandle_20x15.png