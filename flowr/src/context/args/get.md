## Args (//context/args/get)
Get the arguments the flow was executed with

### Definition
```toml
{{#include get.toml}}
```

### Include using
```toml
[[process]]
alias = "get"
source = "context://args/get"
```

### Inputs

### Output
* text - Array of Strings of the command line arguments the flow was invoked with.
* json - Array of Json parsed values of the command line arguments the flow was invoked with.