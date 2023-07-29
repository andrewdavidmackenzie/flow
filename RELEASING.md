# Releasing with `cargo release`

## What is `cargo release`
[Cargo release](https://github.com/crate-ci/cargo-release) is a `cargo` plugin for releasing your project.

You can find more details in the [repo](https://github.com/crate-ci/cargo-release) but a summary is:
   * Update the version number (based on your input) of all workspace projects and update the version of the 
     dependencies between them 
   * It will publish and release the entire workspace, building and publishing (to crates-io) your crates 
in the correct order based on the dependency tree they form.
   * Modify the Cargo.toml files, commit those changes, git tag and push all to github repo on master branch

## Using `cargo release` locally
This is invoked locally with (where minor indicates to increment the minor part of the versions number:

```
cargo release --no-verify --workspace --execute minor
```

There is a make target in the Makefile to facilitate this:

```
make release
```

## Release Github Actions workflow
Once the local publish actions have completed and the git tags push, then the [release.yaml](.github/workflows/release.yml)
Github Action release workflow takes over, executing in Github.

A summary of what this workflow does is:
   * It creates a DRAFT release in Github
   * It does a build of the project on each of the supported targets (x86_64-unknown-linux-gnu and x86_64-apple-darwin)
   * It uploads to Github the built assets and attaches them the release 
   * It uploads the manifest of contents to the GH Release
   * It removes the DRAFT marker on the release (if all went well)

An example release (before the removal of "flowsamples" crate) can be seen 
[here](https://github.com/andrewdavidmackenzie/flow/releases/tag/v0.92.0). You can see that
there are two assets (macos and linux versions) for each of the member crates , plus 
the usual source code zip & tarball.

## Remaining Work
When publishing to crates.io, each crate is copied to a clean directory and built from scratch and 
tests ran, to make sure it is OK to publish it (and downloaders will be able to compile and install it). 

This is the verification process used by `cargo publish` and `cargo release`

### `flowstdlib` exception
Because building flowstdlib takes so long (compiling many small projects to generate the WASM files 
for each function), it is not compiled to the $OUT_DIR as rust crates should do, but to $HOME/.flow/lib
so that only modified files are re-compiled each time.

That causes the verification process to fail. Hence to publish the `flowstdlib` crate we need to use the
`--no-verify` option to `cargo publish` and hence to `cargo release`

https://github.com/andrewdavidmackenzie/flow/issues/1633

If that issue can be avoided, then the `--no-verify` option can be dropped.

### Projects in the tree but not in the workspace
There are a number of rust cargo projects in the directory tree, that for various reasons
are not part of the workspace, and not sub-crates of others, but disjoint.

An example would be the directories to produce WASM files for provided implementations in
some of the flowr examples ([mandlebrot](flowr/examples/mandlebrot/pixel_to_point/Cargo.toml)
and [reverse-echo](flowr/examples/reverse-echo/reverse/Cargo.toml)), and each of 
the functions in the flowstdlib (e.g. [add](flowstdlib/src/math/add/Cargo.toml)).

These projects have dependencies on `flowcore` and `flowmacro` but the release process
will not see them as members of the project and is unable to update the version numbers.

As an interim solution, the version number specified is very open (e.g. version = "0"),
so that any release with the same major version number will be picked up and used after it
is published to crates.io.

Whenever a major version number bump is done in the future, these will have to be edited 
manually prior to the release build/verify project.

### Github action to do the local process also
A github action could be added to do the local publish step described above in
"Using `cargo release` locally", still triggered manually when you are ready to do a release
but via github UI, saving the local build/verify. This would need secrets to be able to 
publish to crates.io.