## To Number (//flowstdlib/fmt/to_number.toml)
Convert an input type to a String

#### Definition
```
name = "ToString"

[[input]]

[[output]]
type = "String"
```

#### Include using
```
[[process]]
alias = "to_string"
source = "lib://flowstdlib/fmt/to_string.toml"
```

#### Input
* The data to convert to a String. Current types supported are:
* String - a bit redundant, but it works
* Bool - Boolean JSON value
* Number - A JSON Number
* Array - An JSON array of values that can be converted, they are converted one by one

#### Output
* The String equivalent of the input value