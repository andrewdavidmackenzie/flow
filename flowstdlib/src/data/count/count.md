## Count (//flowstdlib/data/count)
Takes a value on it's input and sends the same value on it's output and adds one to the count
received on 'count' input and outputs new count on 'count' output

### Definition
```toml
{{#include count.toml}}
```

### Include using
```toml
[[process]]
source = "lib://flowstdlib/data/count"
```