## OrderedSplit (//flowstdlib/data/ordered_split)
Split a string into (possibly) its constituent parts based on a separator.

It guarantees to produce an array of strings, ordered the same as the input string.

### Definition
```toml
{{#include ordered_split.toml}}
```

### Include using
```toml
[[process]]
source = "lib://flowstdlib/data/ordered_split"
```