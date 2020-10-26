## Args (//flowruntime/args/get)
Get the arguments the flow was executed with

### Definition
```toml
{{#include get.toml}}
```

### Include using
```toml
[[process]]
alias = "get"
source = "lib://flowruntime/args/get"
```

### Inputs

### Output
* Array of Strings of the command line arguments the flow was invoked with.