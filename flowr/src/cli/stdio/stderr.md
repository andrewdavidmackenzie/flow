## Stderr (//context/stdio/stderr)
Output text to the STDERR of the process invoking the flow. If an array is passed then each element
is output on a separate line.

### Include using
```toml
[[process]]
source = "context://stdio/stderr"
```

### Input
* (default) - the object to output a String representation of (string, boolean, number, array)

#### Output