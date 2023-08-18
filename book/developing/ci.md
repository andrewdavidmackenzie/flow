### Continuous Integration testing of flow
The CI build and test run on each push to a branch or PR include unit and integration tests, rust doc-tests and it 
also compiles, generates, runs and checks the output of all the examples in `flowr`

It also checks that clippy passes without warnings and that the book builds and does not have any broken links.

Before pushing to Github, you should make sure that `make` passes.