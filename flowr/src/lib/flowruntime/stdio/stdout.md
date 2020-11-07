## Stdout (//flowruntime/stdio/stdout)
Output text to the STDOUT of the process invoking the flow. If an array is passed then each element
is output on a separate line.

### Definition
```toml
{{#include stdout.toml}}
```

### Include using
```toml
[[process]]
alias = "stdout"
source = "lib://flowruntime/stdio/stdout"
```

### Input
* (default) - the object to output a String representation of (String, Boolean, Number, Array)

### Output