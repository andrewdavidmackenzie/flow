## Args (//context/args/get)
Get the arguments the flow was executed with

### Include using
```toml
[[process]]
alias = "get"
source = "context://args/get"
```

### Inputs

### Output
* string - Array of Strings of the command line arguments the flow was invoked with.
* json - Array of Json parsed values of the command line arguments the flow was invoked with.