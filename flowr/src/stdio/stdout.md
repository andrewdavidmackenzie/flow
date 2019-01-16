## Stdout (//flowr/stdio/stdout.toml)
Output text to the STDOUT of the process invoking the flow. If an array is passed then each element
is output on a separate line.

#### Include using
```
[[process]]
alias = "stdout"
source = "lib://flowr/stdio/stdout.toml"
```

#### Input
* (default) - the object to output a String representation of (String, Boolean, Number, Array)

#### Output