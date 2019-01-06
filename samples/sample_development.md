### Developing a sample
To develop a new sample, just create a new folder under 'samples' with your sample name. 

Add the context.toml and any other included flows and describe them.

Add a DESCRIPTION.md file that describes what the sample does and what features of flow it uses.

Add an entry in the guide's "samples" section that will include the DESCRIPTION.md file above.

The Makefile will automatically discover it and attemtp to test it using the following files which
you should add:
* test_arguments.txt - arguments passed to the flow on the command line when executing it
* test_input.txt - test input to send to the sample flow using STDIN
* expected_output.txt - the output you expect to be sent to STDOUT by the flow when running correctly

### Test the new sample
From the root project folder you can ask make to test just this sample using:
`make samples/{new_sample_folder}/test_output.txt`

Make will compile and generate the sample flow, then run it using the arguments supplied in the test_arguments.txt
file, sending it the input found in the test_input.txt file and capturing the output in test_output.txt 
file and finally comparing that output to the expected correct output specified in expected_output.txt.

That make target depends on the 'compiler' Makefile target, so it will recompile 'flowc' and any 
dependencies before it compiles, generates, builds and tests the sample in question.

You can debug the execution of a flow using the method described in 'Debugging your first flow' section
in the guide.