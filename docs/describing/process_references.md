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
    * e.g. `source = "my_function"`
    * e.g. `source = "my_flow"`
    * e.g. `source = "subdir/my_other_function"`
    * e.g. `source = "subdir/my_other_process"`
* in a different flow's directories, using relative file paths
    * e.g. `source = "../other_flow/other_function"`
    * e.g. `source = "../other_flow/other_flow"`
* elsewhere in the local file system, using absolute paths
    * e.g. `source = "/root/other_directory/other_function"`
    * e.g. `source = "/root/other_directory/other_flow"`

#### Remote Web Server
The process definition file can be found on a remote server, just specify the 
URL of the file:
* e.g. `source = "http://my_flow_server.com/folder/function"`
* e.g. `source = "https://my_secure_flow_server.com/folder/flow"`

#### Library Processes
The process is in a library that is available to your current installation. 
In order for flow to find the function at compile time it uses the 
environment variable `FLOW_LIB_PATH`, that is a `PATH` style variable with zero or
more directory entries or URLs separated by the `","` character
* e.g. `source = "lib://flowstdlib/math/add"`
    * Library name = `flowstdlib`
    * Function path within the library = `math/add`
    
All the directories in the path are searched for a top-level sub-directory that 
matches the library name.

If the named library is found, the Function path within the library is used to try and 
find the process definition file.

For example, if I define `FLOW_LIB_PATH` thus:
* `FLOW_LIB_PATH=/Users/me/workspace/flow`

And my flow references a process thus:
```toml
[[process]]
alias = "sum"
source = "lib://flowstdlib/math/add"
```

Then the directory `/Users/me/workspace/flow/flowstdlib` is looked for.

If that directory and hence the library is found, then the Function path within the library
`stdio/stdin` is used to create the full path to the Function definition file 
`/Users/me/workspace/flow/flowstdlib/math/add`.

If that file exists and can be read, the process defined there is used and 
included in the flow.

### Initializing an IO in a reference
An IO of a reference process may be initialized with a value, in one of two ways:
* `once` - the value is inserted into the IO on startup only and there after it will remain empty if a value is not 
sent to it from a Process
* `always` - the value will be inserted into the IO each time it is empty, of there is not a value already
sent from a process.

When a process only has one input, and it is not named, then you can refer to it by the name
`default` for the purposes of specifying an initializer

Eamples:
```toml
[[process]]
alias = "print"
source = "lib://flowruntime/stdio/stdout"
input.default = {once = "Hello World!"}
```

```toml
[[process]]
alias = "second-start"
source = "lib://flowstdlib/fmt/to_json"
input.default = {always = "2"}
```