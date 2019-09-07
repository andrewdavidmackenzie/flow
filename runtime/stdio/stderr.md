## Stderr (//runtime/stdio/stderr)
Output text to the STDERR of the process invoking the flow. If an array is passed then each element
is output on a separate line.

#### Include using
```
[[process]]
alias = "stderr"
source = "lib://runtime/stdio/stderr"
```

#### Input
* (default) - the object to output a String representation of (String, Boolean, Number, Array)

#### Output