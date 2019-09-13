## Important make targets
- (default) ```make``` will build, run local tests and generate docs.
- ```make build-guide``` will just generate the HTML for the guide if you are writing docs. But better
would be to just ```cd guide && mdbook serve``` as that will track and update the generated content as 
you make changes, allowing you to view them instantly with a browser refresh.
- ```make test``` this should be what you run to check changes you have made work OK. At the moment it is the 
same as 'make travis' until I re-instate some tests I was having issues with.

## Other less important make targets
- ```make package``` this will prepare 'flowc' for publishing to crates.io, but this shouldn't be needed by 
most people. It should also package the ide (when I fix that!).

- ```make clean``` this will clean all generated files that 'cargo' knows about as well as the generated code and test 
output files for each of the samples, and should leave everything clean.