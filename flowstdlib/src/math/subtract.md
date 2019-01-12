## Subtract (//flowstdlib/math/subtract.toml)
Subtract one input from another to produce a new output

#### Definition
```
name = "Subtract"

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
[[process]]
alias = "subtract"
source = "lib://flowstdlib/math/subtract.toml"
```

#### Inputs
* `i1` - first input of type `Number`
* `i2` - second input of type `Number`

#### Outputs
* `i1` minus `i2` of type `Number`