## Sequence (//flowstdlib/math/sequence.toml)
Generate a sequence of numbers between a start and end number that is supplied

#### Include using
```
[[process]]
alias = "sequence"
source = "lib://flowstdlib/math/sequence.toml"
```

#### Inputs
* `start` - the first number of the sequence to generate, type `Number`
* `end` - the last number of the sequence, type `Number`

#### Outputs
* `sequence` the output sequence of type `Number`
* `min` the minimum of the sequence, re-output for use downstream, type `Number`
* `max` the limit of the sequence, re-output for use downstream, type `Number`
* `done` a signal of value `true` that is output when the sequence ends, type `Bool`