# Contributing

There are many ways to contribute:
- Report bugs or suggest features by creating an [issue](https://github.com/andrewdavidmackenzie/flow/issues)
- Pick up an existing issue and submit a fix
- Add or improve documentation, examples, or tests
- Improve the compiler, runtime, or standard library

## Getting Started
1. Fork the [repo](http://github.com/andrewdavidmackenzie/flow) and clone locally
2. Build from source (see [Developing flow](overview.md))
3. Run an example or two (fibonacci is a good starting point)
4. Choose an issue from the [GitHub project](https://github.com/users/andrewdavidmackenzie/projects/2/views/1)

## Working on an Issue
- If no issue exists for your change, create one first
- Add a comment to the issue so others know you're working on it
- Create a branch for the issue in your fork
- Make your changes and update tests, docs, and examples as needed

## Before Pushing
Run the full build and test locally:
```bash
make
```

This checks that code builds, clippy passes without warnings, all tests pass,
and the book builds with valid links.

## Submitting a PR
- Reference the issue in your PR description (e.g., "Fixes #123")
- Wait for CI to pass on all platforms
- For work-in-progress, prefix the PR title with "WIP - "

## Continuous Integration
CI runs on every push to a branch or PR and checks:
- Unit tests, integration tests, and doc-tests
- All examples compile, run, and produce expected output
- Clippy passes without warnings
- The book builds with no broken links

## Contact
If in doubt, reach out via:
- [GitHub issues](https://github.com/andrewdavidmackenzie/flow/issues)
- Email: andrew@mackenzie-serres.net
- Matrix: andrewdavidmackenzie:matrix.org
