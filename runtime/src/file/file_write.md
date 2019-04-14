## Write (//runtime/file/write.toml)
Writes `bytes` of data supplied to the file named `filename`, creating it if necessary.

#### Include using
```
[[process]]
alias = "write"
source = "lib://runtime/file/write.toml"
```

#### Inputs
* `bytes` - the data to be written to the file
* `filename` - String with the name of the file to be written, absolute or relative to the current working
directory of the process invoking the flow.

#### Outputs