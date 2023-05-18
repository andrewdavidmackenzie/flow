## Read (//context/file/file_read)
Reads `bytes` of data from the file with path `path`

### Include using
```toml
[[process]]
source = "context://file/file_read"
```

### Inputs
* `path` - String with the path of the file to be read, absolute (starting with `/`) or relative to the current working
directory of the process invoking the flow.

#### Outputs
* `bytes` - the raw data data read from the file
* `string` - the data read from the file, as a string
* `path` - String with the path of the file that was read, as was passed to the input.
