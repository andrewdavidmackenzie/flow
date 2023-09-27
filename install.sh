#!/usr/bin/env sh

# Install cargo-binstall
curl -L --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# binstall the flowc binary
cargo binstall flowc
echo "flowc binary installed by cargo"

# cargo binstall flowr crate's multiple binaries: flowrcli, flowrgui and flowrex

# download the flowstdlib artifact and expand into $HOME/.flow/lib
mkdir -p "$HOME"/.flow/lib
curl -L --tlsv1.2 -sSf https://github.com/andrewdavidmackenzie/flow/releases/download/v0.135.0/flowstdlib-v0.135.0.tar.xz | tar -x --directory "$HOME"/.flow/lib
echo "flowstdlib library installed in $HOME/.flow/lib"

# download the flowrcli context into $HOME/.flow/runner
mkdir -p "$HOME"/.flow/runner
curl -L --tlsv1.2 -sSf https://github.com/andrewdavidmackenzie/flow/releases/download/v0.135.0/flowrcli-v0.135.0.tar.xz | tar -x --directory "$HOME"/.flow/runner
echo "flowrcli runner context installed in $HOME/.flow/runner"

# download the flowrgui context into $HOME/.flow/runner
mkdir -p "$HOME"/.flow/runner
curl -L --tlsv1.2 -sSf https://github.com/andrewdavidmackenzie/flow/releases/download/v0.135.0/flowrgui-v0.135.0.tar.xz | tar -x --directory "$HOME"/.flow/runner
echo "flowrgui runner context installed in $HOME/.flow/runner"
