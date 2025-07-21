all: build test time bench

build:
	cargo build --release

test:
	cargo test

time: build
	time target/release/mandlebrot mandle_20x15.png 20x15 -1.20,0.35 -1,0.20
	open mandle_20x15.png

bench:
	cargo bench

clean:
	rm -f mandle_20x15.png