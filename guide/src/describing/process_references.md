## Process Reference
Flows may reference a another flow or a function which is defined in a separate 
definition file. These are referred to under the generic term of `process`
 
### Process Reference Fields
* `alias` - an alias to use to refer to the process in this flow.
    * This can be different from the `name` defined by the process itself
    * This can be used to create two difference instances of a process in a flow, 
    and the ability to refer to them separately and distinguish them in connections.
* `source` - the source of where the process is defined. 

### Source formats
The following formats for specifying the `source` are available:
* No "scheme" in the URI --> `file:` is assumed
* `file:` scheme --> look for process on the Local File System
* `http:` or `https:` scheme --> look for process on a Remote Web Server
* `lib:` --> look for process in a Library

#### Local File System
The process definition file is in the local file system.
* in the flow's directories, using relative file paths 
    * e.g. `source = "my_function.toml"`
    * e.g. `source = "my_flow.toml"`
    * e.g. `source = "subdir/my_other_function.toml"`
    * e.g. `source = "subdir/my_other_process.toml"`
* in a different flow's directories, using relative file paths
    * e.g. `source = "../other_flow/other_function.toml"`
    * e.g. `source = "../other_flow/other_flow.toml"`
* elsewhere in the local file system, using absolute paths
    * e.g. `source = "/root/other_directory/other_function.toml"`
    * e.g. `source = "/root/other_directory/other_flow.toml"`

#### Remote Web Server
The process definition file can be found on a remote server, just specify the 
URL of the file:
* e.g. `source = "http://my_flow_server.com/folder/function.toml"`
* e.g. `source = "https://my_secure_flow_server.com/folder/flow.toml"`

#### Library Processes
The process is in a library that is available to your current installation. 
In order for flow to find the function at compile time it uses the 
environment variable `FLOW_LIB_PATH`, that is a `PATH` style variable with zero or
more directory entries. 
* e.g. `source = "lib://runtime/stdio/stdin.toml"`
    * Library name = `flowrlib`
    * Function path within the library = `stdio/stdin.toml`
    
All the directories in the path are searched for a top-level sub-directory that 
matches the library name.

If the named library is found, the Function path within the library is used to try and 
find the process definition file.

For example, if I define `FLOW_LIB_PATH` thus:
* `FLOW_LIB_PATH=/Users/me/workspace/flow`

And my flow references a process thus:
```
[[process]]
alias = "stdin"
source = "lib://runtime/stdio/stdin.toml"
```

Then the directory `/Users/me/workspace/flow/flowrlib` is looked for.

If that directory and hence the library is found, then the Function path within the library
`stdio/stdin.toml` is used to create the full path to the Function definition file 
`/Users/me/workspace/flow/flowrlib/stdio/stdin.toml`.

If that file exists and can be read, the process defined there is used and 
included in the flow.
