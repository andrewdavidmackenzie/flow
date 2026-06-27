## ToString (//flowstdlib/fmt/to_string)
Convert any JSON value to its string representation.

Takes a generic input value and outputs its string form using JSON
serialization. This is useful for converting numbers, arrays, objects,
or other types into strings for display or further processing.

### Input
* Any JSON value (generic input — accepts all types)

### Output
* `string` — the string representation of the input value

### Supported conversions
* `null` → `"null"`
* `true` / `false` → `"true"` / `"false"`
* `42` → `"42"`
* `"hello"` → `"\"hello\""`
* `[1,2,3]` → `"[1,2,3]"`
* `{"key": 42}` → `"{\"key\":42}"`

### Include using
```toml
[[process]]
source = "lib://flowstdlib/fmt/to_string"
```