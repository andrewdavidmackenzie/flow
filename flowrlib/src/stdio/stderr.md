## Stderr (//flowrlib/stdio/stderr.toml)
Output text to the STDERR of the process invoking the flow. If an array is passed then each element
is output on a separate line.

#### Definition
```
name = "Stderr"

[[input]]
```

#### Include using
```
[[function]]
alias = "stderr"
source = "lib://flowrlib/stdio/stderr.toml"
```

#### Input
* (default) - the object to output a String representation of (String, Boolean, Number, Array)

#### Output