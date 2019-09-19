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

##### Commands
# See https://www.christianengvall.se/electron-packager-tutorial/
package: package-mac package-win package-debian

installer-mac: ./installers/flow-ide.dmg
	@echo "------- DMG installer for MacOS can be found at ./installers/flow-ide.dmg  -----------"

package-mac: ./release-builds/Flow\ IDE-darwin-x64
	@echo "------- MacOS package can be foune at ./release-builds/Flow IDE-darwin-x64   -----------\n"

##### Dependencies
./release-builds/Flow\ IDE-darwin-x64: build
	@echo "------- Packaging for MacOS                      -----------"
	@electron-packager . --overwrite --platform=darwin --arch=x64 --icon=assets/icons/mac/128x128.icns --prune=true --out=release-builds

package-win: build
	@echo "------- Packaging for Windows                    -----------"
	@electron-packager . flowide --overwrite --asar=true --platform=win32 --arch=ia32 --icon=assets/icons/win/128x128.ico --prune=true --out=release-builds --version-string.CompanyName=CE --version-string.FileDescription=CE --version-string.ProductName="Flow IDE"
	@echo "------- Packaged for Windows in ./release-builds -----------"

package-debian: build
	@echo "------- Packaging for Debian                     -----------"
	@electron-packager . flowide --overwrite --asar=true --platform=linux --arch=x64 --icon=assets/icons/png/128x128.png --prune=true --out=release-builds
	@echo "------- Packaged for Linux in ./release-builds   -----------"

# see: https://www.christianengvall.se/dmg-installer-electron-app/
./installers/flow-ide.dmg: ./release-builds/Flow\ IDE-darwin-x64
	@echo "\n------- Creating DMG installer for MacOS                  -----------"
	electron-installer-dmg --overwrite ./release-builds/Flow\ IDE-darwin-x64/Flow\ IDE.app installers/flow-ide

installer-debian: package-debian
	@echo "------- Created installer for Debian                   -----------"
	electron-installer-debian --arch amd64 --config debian.json
	@echo "------- Created installer for Debian in ./installers/  -----------"

clean:
	@cargo clean
	@rm -rf pkg target Cargo.lock .bin package-lock.json installers release-builds node_modules
