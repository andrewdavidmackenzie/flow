### What file to select as the Context?
`flowc` determines the flow to run using one of these three methods:

* no filename or directory passed, so it looks for "context.toml" in the current directory
* a filename or directory is passed:
    * if the parameter passed is a directory, it looks for "context.toml" in that directory
    * if a filename is passed, it uses that as the flow to load
    
Then it tries to load the flow hierarchy from the context flow found, or reports an error if 
none was found.

TODO: Document URL format and lack of folder option.
