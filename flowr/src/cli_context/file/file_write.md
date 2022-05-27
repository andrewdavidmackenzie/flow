## Write (//context/file/write)
Writes `bytes` of data supplied to the file named `filename`, creating it if necessary.

### Definition
```toml
{{#include file_write.toml}}
```

### Include using
```toml
[[process]]
alias = "write"
source = "context://file/write"
```

### Inputs
* `bytes` - the data to be written to the file
* `filename` - String with the name of the file to be written, absolute or relative to the current working
directory of the process invoking the flow.

#### Outputs