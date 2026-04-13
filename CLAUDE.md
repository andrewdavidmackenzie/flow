# Project

Flow resides at https://github.com/andrewdavidmackenzie/flow, and it is a data flow programming
system. You can read more in the README.md and other Markdown files in the root folder 
or the extensive docs in the ./book folder.


## General Considerations

1. Allow Claude to say "I don't know" if it can't find information to confirm a
   conclusion or answer, or can't quote sources for a statement when needed. I
   prefer no answer than one that may mislead us.

2. Verify with Citations. Make sure you can explain any conclusions you have reached
   by being able to cite the source information and then explain the logic used.

3. Use direct quotes for factual grounding.

## Workflow Rules

- Never commit to master/main branch, always use a feature branch and create a PR.
- Always wait for code reviews to terminate or be repeated if they failed due to
  rate limiting, and then address all comments from the review.
- Always wait for the human user to approve before you merge a PR.
- Don't close GitHub issues without the user's explicit approval.
- Don't change Rust versions or install or uninstall anything using rustup without the user's explicit approval.
- Don't add new crate dependencies without the user's explicit approval.
- Always run `make test` (not just `cargo test`) before pushing,
  since the Makefile builds nested workspaces (like `examples/workspace_test`) that aren't part of the
  main workspace.
- Always run `make clippy` and `cargo fmt` before committing or pushing changes.
- Explain your analysis of the problem and proposed implementation plan before starting to
  implement changes. Describe what files will be modified, what functions will be added/deleted/modified

## General rust best practices
- keep visibility of structs and functions to the most private possible, pub – pub(crate) – private
- before adding a new struct or function, scan the code base for similar functions and structs and attempt to
  reuse them if they can be with minimal changes

## Coding Rules

- **macOS and Linux on aarch64 and x86_64** — jonesy supports macOS and Linux on both aarch64 and x86_64.
  Don't add support for other architectures or operating systems.
- Use rust canonical code where possible. Implement `From` traits for conversion, create structs
  with methods, use traits when multiple implementations may be needed, etc.

## Testing Rules

- Don't assume that any test failure is independent of your change. We usually start
  a new feature branch from master where tests were working.
- Use `make test` not `cargo test`
- Don't modify any "expected" file in a test to make a test pass without first shwoing a comparison 
of the two to the user, or showing both side by side, and then the user explicitly approving the
replacement of the old one with the new one.

## Committing and Pushing

- Never consider a task done, nor attempt to commit or push a change until make test passes.
