## Divide (//flowstdlib/math/divide.toml)
Divide one input by another, producing outputs for the dividend, divisor, result and the remainder

#### Include using
```
[[process]]
alias = "divide"
source = "lib://flowstdlib/math/divide.toml"
```

#### Inputs
* `dividend` - the number to be divided, of type `Number`
* `divisor` - the number to divide by, of type `Number`

#### Outputs
* `dividend` - re output the `dividend` input, of type `Number`
* `divisor` - re output the `divisor` input, of type `Number`
* `result` - the result of the division, of type `Number`
* `remainder` - the remainder of the division, of type `Number`