## Write (//context/file/file_write)
Writes `bytes` of data supplied to the file named `filename`, creating it if necessary.

### Include using
```toml
[[process]]
source = "context://file/file_write"
```

### Inputs
* `bytes` - the data to be written to the file
* `filename` - String with the name of the file to be written, absolute or relative to the current working
directory of the process invoking the flow.

#### Outputs