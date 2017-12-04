
Check connections all match up in direction.

Write tests for compile functions and connection mapping and dropping etc

Maybe add references to values and functions in the connections when we are doing that

Then create table of values and table of functions

Using connections, for each output, add a reference to one or more input that data should be sent to when made available.

Code Improvements
================
Look at methods in flow and loader and see how many of the ones that look for io etc
could be returnng references and not creating new strings with format!

Look to see how connection table sin compile could be done with just references and not creating
all those new strings and new vectors.