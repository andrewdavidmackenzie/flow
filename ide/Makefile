app: build
	@echo "------------ Starting 'ide' as electron app ----------------"
	@npm run app

web: build
	@echo "------------ Starting 'ide' as web app ---------------------"
	@npm run start

build: npm-install
	@echo "------------ Starting npm-build for 'ide' ------------------"
	@npm run build
	@echo "------------ Finished building 'ide' -----------------------\n"

npm-install:
	@npm install

test:
	@echo ""
	@echo "--- Started  'cargo build' for 'ide' on wasm target ---"
	@cargo build --target wasm32-unknown-unknown
	@echo "--- Ended    'cargo build' for 'ide' on wasm target ---"
	@echo ""
	@echo "--- Started  'cargo test' for 'ide' on native target ---"
	@cargo test
ifeq ($(TRAVIS),)
	@wasm-pack test --chrome --headless -- --features wasm-bindgen
endif
	@echo "--- Finished 'cargo test' for 'ide/crate' on native target ---"

package:
	@echo ""
	@echo "------- Started  packaging ide -----------"
	@echo "Not implemented yet"
	@echo "------- Finished packaging ide -----------"

clean:
	cargo clean
	rm -rf pkg target Cargo.lock wasm-pack.log dist .bin package-lock.json