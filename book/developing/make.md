## Important make targets
- (default) ```make``` will build, run local tests and build the book and check links are valid

other targets you can run to perform only a part of the whole build 
- ```make build``` will build the libs and binaries
- ```make clippy``` will run clippy on all code including tests
- ```make test``` will run all tests, including testing flowr examples run and pass
- ```make book``` will build the book and check all links are valid