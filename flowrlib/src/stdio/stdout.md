## Stdout (//flowrlib/stdio/stdout.toml)
Output text to the STDOUT of the process invoking the flow. If an array is passed then each element
is output on a separate line.

#### Definition
```
name = "Stdout"

[[input]]
```

#### Include using
```
[[process]]
alias = "stdout"
source = "lib://flowrlib/stdio/stdout.toml"
```

#### Input
* (default) - the object to output a String representation of (String, Boolean, Number, Array)

#### Output