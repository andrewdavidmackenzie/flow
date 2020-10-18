## Enumerate (//flowstdlib/data/enumerate)
Enumerate the elements of an Array

With an input array such as `["a", "b"]` it will assign an index to each element
and produce an output array of tuples (Array of two elements) such as `[[0, "a"], [1, "b"]]`

### Definition
```toml
{{#include enumerate.toml}}
```

### Include using
```toml
[[process]]
source = "lib://flowstdlib/data/enumerate"
```