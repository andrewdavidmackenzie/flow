## Tap (//flowstdlib/control/join.toml)
Control the flow of a piece of data by waiting for a second value to be available

#### Include using
```
[[process]]
alias = "join"
source = "lib://flowstdlib/control/join.toml"
```

#### Inputs
* `data` - the data we wish to control the flow of
* `control` - a second value we wait on

#### Outputs
* `data`