### Continuous Integration testing of flow
The CI build and test run in Travis on each push to a branch or PR can be run locally 
using ```make travis```.

These tests include unit and integration tests, rust doc-tests and it also compiles, generates, runs and checks the 
output of all the samples found in the ./samples folder.

This will also rebuild the guide locally.

I recommend to make sure this runs without errors and passes before pushing to GitHub.
