## Stdin (//flowr/stdio/stdin.toml)
Read text from the STDIN of the process invoking the flow until EOF is detected, after which it will not run
again.

#### Include using
```
[[process]]
alias = "stdin"
source = "lib://flowr/stdio/stdin.toml"
```

#### Inputs

#### Output
* Text read from STDIN - with leading and trailing whitespace (including EOF) trimmed.