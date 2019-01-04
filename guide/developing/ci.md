### CI tests
The CI build and test run in Travis on each push to a branch or PR can be run locally 
using ```make travis```.

These tests include unit and integration tests, rust doc-tests and it also compiles, generates, runs and checks the 
output of all the samples found in the ./samples folder.

I recommend making sure this runs fine and passes before pushing to GitHub.