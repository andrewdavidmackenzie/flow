### Testing a failing sample
If you have made a change (in source of compiler, or a sample definition) that is causing that sample to fail,
then you can easily run just the compile and test of that sample using a make target such as:
- ```make samples/fibonacci/test_output.txt```

where 'finonacci' is the name of the sample you want to test.

That make target depends on 'compiler' so it will make sure to recompile 'flowc' and any dependencies before it 
compiles, generates, builds the sample in question. It then runs the sample with pre-defined inputs and captures 
the output and compares it to previously generated "correct" output - passing if they are the same and failing if not.
