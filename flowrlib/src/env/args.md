## Args (//flowrlib/env/args.toml)
Get the arguments the flow was executed with

#### Definition
```
name = "Args"

[[output]]
type = "Array/String"
```

#### Include using
```
[[process]]
alias = "args"
source = "lib://flowrlib/env/args.toml"
```

#### Inputs

#### Output
* Array of Strings of the command line arguments the flow was invoked with.