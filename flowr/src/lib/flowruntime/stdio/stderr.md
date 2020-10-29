## Stderr (//flowruntime/stdio/stderr)
Output text to the STDERR of the process invoking the flow. If an array is passed then each element
is output on a separate line.

### Definition
```toml
{{#include stderr.toml}}
```

### Include using
```toml
[[process]]
alias = "stderr"
source = "lib://flowruntime/stdio/stderr"
```

### Input
* (default) - the object to output a String representation of (String, Boolean, Number, Array)

#### Output