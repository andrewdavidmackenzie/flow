## Write (//flowrlib/file/write.toml)
Writes `bytes` of data supplied to the file named `filename`, creating it if necessary.

#### Definition
```
name = "Write"

[[input]]
name = "filename"
type = "String"

[[input]]
name = "bytes"
type = "Array"
```

#### Include using
```
[[function]]
alias = "write"
source = "lib://flowrlib/file/write.toml"
```

#### Inputs
* `bytes` - the data to be written to the file
* `filename` - String with the name of the file to be written, absolute or relative to the current working
directory of the process invoking the flow.

#### Outputs