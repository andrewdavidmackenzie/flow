app: wasm-pack-build-bundler npm-build npm-install
	@echo "------------ Starting 'ide' as electron app ----------------"
	@npm start

web: wasm-pack-build-web npm-build
	@echo "------------ Starting 'ide' as web app ---------------------"
	@npm run web

npm-build: npm-install
	@echo "------------ Starting npm-build for 'ide' ------------------"
	@npm run build
	@echo "------------ Finished building 'ide' -----------------------\n"

wasm-pack-build-bundler: npm-install
	@echo "------------ Started wasm-pack bundler build ---------------"
	@wasm-pack build --dev --target bundler crate/
	@echo "------------ Finished wasm-pack build to crate/pkg ---------\n"

wasm-pack-build-web: npm-install
	@echo "------------ Started wasm-pack web build ---------------"
	@wasm-pack build --dev --target web crate/
	@echo "------------ Finished wasm-pack build to crate/pkg ---------\n"

npm-install:
	@npm install

test:
	@echo ""
	@echo "--- Started  'cargo build' for 'ide/crate' on wasm target ---"
	@cd crate;cargo build --target wasm32-unknown-unknown
	@echo "--- Ended    'cargo build' for 'web/crate' on wasm target ---"
	@echo ""
	@echo "--- Started  'cargo test' for 'ide/crate' on native target ---"
	@cd crate;cargo test
ifeq ($(TRAVIS),)
	@cd crate;wasm-pack test --chrome --headless -- --features wasm-bindgen
endif
	@echo "--- Finished 'cargo test' for 'ide/crate' on native target ---"

package:
	@echo ""
	@echo "------- Started  packaging ide -----------"
	@echo "Not implemented yet"
	@echo "------- Finished packaging ide -----------"

clean:
	cargo clean
	rm -rf crate/bin crate/pkg/* crate/target crate/Cargo.lock crate/wasm-pack.l	og dist .bin