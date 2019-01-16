## Stdio (//flowr/stdio)
Functions to interact with the Environment, related to standard input and output (and error).

The values sent to these functions are read from standard input of the process that launched the flow
causing the function to block until input (or EOF) is detected. Output is printed on the STDOUT/STDERR
of the process invoking the flow.