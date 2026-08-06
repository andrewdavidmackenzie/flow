---
name: start-new-issue
description: Starts work on a new issue from the issue list in a project. Use when starting work on a new github issue. Requires git, gh.
---

When starting work on a new issue from GitHub for a project:

## New Branch named for the issue

Make sure that the local master or main branch is up to date with origin by doing a git pull.

If the local code is not currently on master or main branch then ask the user if they would like to continue the 
work on the current branch, or leave that branch for later and start a new one or how to proceed.
Never autonomously decide a path that will result in modified file changes being lost.

Then create a new local branch to start work on. The branch name should be a short two or three-word
summary of the issue title, plus the issue number.

Example:
issue url: https://github.com/andrewdavidmackenzie/jonesy/issues/103
issue title: LSP analysis should be re-run when config changes
issue number: 103

Then a branch name could be `analyse_on_config_change_103`

## Working on the branch

As we work on the branch, we will commit changes once pre-commit checks pass and then later
push the branch and create a PR from it.

###

## Pre-commit checks

Before committing new work to a branch, the following checks should pass:

- `make clippy` if that Makefile target exists or `cargo clippy` if not
- `cargo fmt` either to check or directly to reformat code.
- tests pass via `make test` if that Makefile target exists, or `cargo test` if not

Expect the PR checks to check for:
- non-reducing code coverage of the project and the patch (lines changed), add tests to reduce the possibility
  of these checks failing after pushing. Use local test coverage report if possible.
- Code review by a bot like code rabbit. Perform your own code review before pushing and fix issues, to attempt
  to avoid the PR check failing after push.

## When is a PR ready to be merged?

A PR can be considered ready to be merged when:
- There are no local uncommitted changes
- There are no local committed changes that have not been pushed
- All the points in the GH issue have been addressed via committed, pushed and reviewed changes
- There is no code review in progress since the last pushed change
- There are no unaddressed code review comments since the last change was pushed
- All the comments from code reviews have been addressed, even the Nit ones.

If code coverage checks or code review fails in the PR checks, then iterate with fixes until the PR
is green, all checks pass, and it is ready to be considered for merge.

Don't suggest merging until the above list is met.
