run: wasm
	electron-forge start	

wasm:
	make -f ../rust/Makefile wasm

package:
	npm install
	electron-forge make
