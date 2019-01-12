## Compare (//flowstdlib/control/compare.toml)
Compare two input values and output different boolean values depending on if the comparison
is equal, greater than, greater than or equal, less than or less than or equal.

#### Definition
``` 
name = "Compare"

[[input]]
name = "left"

[[input]]
name = "right"

[[output]]
name = "equal"
type = "Bool"

[[output]]
name = "lt"
type = "Bool"

[[output]]
name = "lte"
type = "Bool"

[[output]]
name = "gt"
type = "Bool"

[[output]]
name = "gte"
type = "Bool"
```

#### Include using
```
[[process]]
alias = "compare"
source = "lib://flowstdlib/control/compare.toml"
```

#### Inputs
* `left` - left hand input
* `right` - right hand input

#### Outputs
* `equal` [Boolean] - outputs true if the two values are equal
* `lt` [Boolean] - outputs true if the left hand value is less than the right hand value
* `lte` [Boolean] - outputs true if the left hand value is less than or equal to the right hand value
* `gt` [Boolean] - outputs true if the left hand value is greater than the right hand value
* `gte` [Boolean] - outputs true if the left hand value is greater than or equal to the right hand value