## Process Reference
Flows may reference a another flow or a function (generically referred to as a `process`) which is defined in a
separate definition file. These are "process references"

### Process Reference Fields
* `source` - A Url (or relative path) of a file/resource where the process is defined. 

For example, here we reference a process called `stdout` (see [context functions](context_functions.md))
```toml
[[process]]
source = "context://stdio/stdout"
```

This effectively brings the function into scope with the name `stdout` and it can then be used in connections
as a source or destination of data.

#### Alias for a Process Reference
* `alias` - an alias to use to refer to a process in this flow.
  * This can be different from the `name` defined by the process itself
  * This can be used to create two or more instances of a process in a flow,
    and the ability to refer to them separately and distinguish them in connections.

For example, here the process called `stdout` is aliased as `print`and then can be referred to using `print`in
connections.
```toml
[[process]]
alias = "print"
source = "context://stdio/stdout"
```

#### Source Url formats
The following formats for the `source` Url are available:
* No "scheme" in the URI --> `file:` is assumed. If the path starts with `/` then an absolute path is used. If
the path does not start with `/` then the path is assumed to be relative to the location of the file referring to it.
* `file:` scheme --> look for process definition file on the local file system
* `http:` or `https:` scheme --> look for process definition file on a the web
* `lib:` --> look for process in a Library that is loaded by the runtime. See [flow libraries](flow_libraries.md) for 
more details on how this Url is used to find the process definition file provided by the library.
* `context:` --> a reference to a function in the context, provided by the runner application. See [context 
  functions](context_functions.md) for more details on how the process definition file is used.

#### File source
This is the case when no scheme or the `file://` scheme is used in the `source` Url.
The process definition file is in the same file system as the file referencing it.
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

#### Web Source
When the `http` or `https` Url scheme is used for `source` the process definition file is loaded via http request
to the specified location.
* e.g. `source = "http://my_flow_server.com/folder/function"`
* e.g. `source = "https://my_secure_flow_server.com/folder/flow"`

### Initializing an input in a reference
Inputs of a referenced process may be initialized, in one of two ways:
* `once` - the value is inserted into the input just once on startup and there after it will remain empty if a 
  value is not sent to it from a Process.
* `always` - the value will be inserted into the input each time after the process runs.

Example, initializing the `add` function's `i1` and `ì2` inputs to 0 and 1 respectively, just once at the start
of the flow's execution.
```toml
[[process]]
source = "lib://flowstdlib/math/add"
input.i1 = { once =  0 }
input.i2 = { once =  1 }
```

Example, initializing the `add` function's `i1` input to 1 every time it runs. The other input is free to be
used in connections and this effectively makes this an "increment" function that adds one to any value sent to it
on the `i2` input.
```toml
[[process]]
source = "lib://flowstdlib/math/add"
input.i1 = { always =  1 }
```

#### Initializing the default input
When a process only has one input, and it is not named, then you can refer to it by the name `default` for the
purposes of specifying an initializer

Example, initializing the sole input of `stdout` context function with the string "Hello World" just once at
the start of flow execution:
```toml
[[process]]
source = "context://stdio/stdout"
input.default = {once = "Hello World!"}
```

