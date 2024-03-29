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
cargo release --workspace --execute minor
```

There is a make target in the Makefile to facilitate this:

```
make release
```

## Release Github Actions workflow
Once the local publish actions have completed and the git tags pushed, then the [release.yaml](.github/workflows/release.yml)
Github Action release workflow takes over, executing in Github.

A summary of what this workflow does is:
   * It creates a DRAFT release in Github
   * It does a build of the project for each supported target (x86_64-unknown-linux-gnu and x86_64-apple-darwin)
   * It builds the portable, WASM, version of flowstdlib
   * It uploads to Github the built assets and attaches them the release 
   * It uploads the manifest of contents to the GH Release
   * It removes the DRAFT marker on the release (if all went well)
   * It builds the book and publishes it to github pages

Releases can be found [here](https://github.com/andrewdavidmackenzie/flow/releases). 
You can see that there are two assets (macos and linux versions) for each of the 
member crates , plus the cross-platform WASM flowstdlib and the usual source code
zip & tarball.

## Remaining Work

### Projects in the tree but not in the workspace
There are a number of rust cargo projects in the directory tree, that for various reasons
are not part of the workspace, and not sub-crates of others, but disjoint.

An example would be the directories to produce WASM files for provided implementations in
some of the flowr examples ([mandlebrot](flowr/examples/mandlebrot/pixel_to_point/function.toml)
and [reverse-echo](flowr/examples/reverse-echo/reverse/function.toml)), and each of 
the functions in the flowstdlib (e.g. [add](flowstdlib/src/math/add/function.toml)).

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