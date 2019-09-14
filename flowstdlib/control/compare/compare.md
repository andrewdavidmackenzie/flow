## Compare (//flowstdlib/control/compare)
Compare two input values and output different boolean values depending on if the comparison
is equal, greater than, greater than or equal, less than or less than or equal.

#### Include using
```toml
[[process]]
alias = "compare"
source = "lib://flowstdlib/control/compare"
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