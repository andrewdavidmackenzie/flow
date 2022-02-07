## ToString (//flowstdlib/fmt/to_string)
Convert an input type to a String

Current types supported are:
 * Null - A null will be printed as "Null"
 * Bool - Boolean JSON value
 * Number - A JSON Number
 * String - a bit redundant, but it works
 * Array - An JSON array of values that can be converted, they are converted one by one
 * Object - a Map of names/objects that will also be printed out
 
### Definition
```toml
{{#include to_string.toml}}
```

### Include using
```toml
[[process]]
source = "lib://flowstdlib/fmt/to_string"
```