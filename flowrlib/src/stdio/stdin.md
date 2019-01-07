## Stdin (//flowrlib/stdio/stdin.toml)
Read text from the STDIN of the process invoking the flow until EOF is detected, after which it will not run
again.

#### Definition
```
name = "Stdin"

[[output]]
type = "String"
```

#### Include using
```
[[function]]
alias = "stdin"
source = "lib://flowrlib/stdio/stdin.toml"
```

#### Inputs

#### Output
* Text read from STDIN - with leading and trailing whitespace (including EOF) trimmed.