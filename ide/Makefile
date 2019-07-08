app: npm-build
	@echo "------------ Starting 'ide' as electron app ----------------"
	@npm run app

web: npm-build
	@echo "------------ Starting 'ide' as web app ---------------------"
	@npm run start

npm-build: npm-install
	@echo "------------ Starting npm-build for 'ide' ------------------"
	@npm run build
	@echo "------------ Finished building 'ide' -----------------------\n"

npm-install:
	@npm install

test:
	@echo ""
	@echo "--- Started  'cargo build' for 'ide/crate' on wasm target ---"
	@cargo build --target wasm32-unknown-unknown
	@echo "--- Ended    'cargo build' for 'web/crate' on wasm target ---"
	@echo ""
	@echo "--- Started  'cargo test' for 'ide/crate' on native target ---"
	@cd crate;cargo test
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