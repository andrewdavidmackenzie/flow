## Add (//flowstdlib/math/add.toml)
Add two inputs to produce a new output

#### Definition
```
name = "Add"

[[input]]
name = "i1"
type = "Number"

[[input]]
name = "i2"
type = "Number"

[[output]]
type = "Number"
```

#### Include using
```
[[function]]
alias = "add"
source = "lib://flowstdlib/math/add.toml"
```

#### Inputs
* `i1` - first input of type `Number`
* `i2` - second input of type `Number`

#### Outputs
* Sum of `i1` and `i2` of type `Number`