## Array Extract (//flowstdlib/data/array_extract)
Extract a sub-array (slice) from an array using start and end indices.

Supports negative indices (Python-style):
- `-1` refers to the last element
- `-N` refers to the Nth element from the end

The `end` index is exclusive (like Rust ranges).

### Common patterns

| Pattern | start | end | Result |
|---------|-------|-----|--------|
| Drop first | 1 | len | `[b, c, d]` from `[a, b, c, d]` |
| Drop last | 0 | -1 | `[a, b, c]` from `[a, b, c, d]` |
| Last N | -N | len | last N elements |
| First N | 0 | N | first N elements |

### Include using
```toml
[[process]]
source = "lib://flowstdlib/data/array_extract"
```
