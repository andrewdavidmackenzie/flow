## Tap (//flowstdlib/control/join)
Control the flow of a piece of data by waiting for a second value to be available

#### Include using
```toml
[[process]]
alias = "join"
source = "lib://flowstdlib/control/join"
```

#### Inputs
* `data` - the data we wish to control the flow of
* `control` - a second value we wait on

#### Outputs
* `data`