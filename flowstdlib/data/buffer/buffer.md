## Buffer (//flowstdlib/data/buffer)
Takes a value on it's input and sends the same value on it's output when it can
run, effectively buffering it until the downstream processs can accept it.

### Definition
```toml
{{#include buffer.toml}}
```

### Include using
```toml
[[process]]
source = "lib://flowstdlib/data/buffer"
```