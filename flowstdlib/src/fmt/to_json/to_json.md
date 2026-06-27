## ToJson (//flowstdlib/fmt/to_json)
Convert a String to a JSON value.

Parses the input string as JSON. If parsing succeeds, outputs the
parsed JSON value (number, array, object, boolean, null, etc.).
If the string is not valid JSON, outputs it as a JSON string value.

### Input
* `string` — the string to parse as JSON

### Output
* The parsed JSON value

### Supported conversions
* `"42"` → number `42`
* `"null"` → JSON null
* `"[1,2,3]"` → array `[1,2,3]`
* `"{\"key\":42}"` → object `{"key":42}`
* `"\"hello\""` → string `"hello"`
* Invalid JSON (e.g., `"-1.20,0.35"`) → returned as a JSON string

### Include using
```toml
[[process]]
source = "lib://flowstdlib/fmt/to_json"
```