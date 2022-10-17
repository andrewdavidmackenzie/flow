## ToString (//flowstdlib/fmt/to_string)
Convert an input type to a String

Current types supported are:
 * null - A null will be printed as "null"
 * boolean - boolean JSON value
 * number - A JSON Number
 * string - a bit redundant, but it works
 * array - An JSON array of values that can be converted, they are converted one by one
 * object - a Map of names/objects that will also be printed out
 
### Definition
```toml
{{#include to_string.toml}}
```

### Include using
```toml
[[process]]
source = "lib://flowstdlib/fmt/to_string"
```