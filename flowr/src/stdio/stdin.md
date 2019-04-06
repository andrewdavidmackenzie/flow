## Stdin (//flowr/stdio/stdin.toml)
Read text from the STDIN of the process invoking the flow until EOF is detected, after which it will not run
again. If you wish to get the value of a line (i.e. after ENTER is pressed, then use [readline](readline.md))

#### Include using
```
[[process]]
alias = "stdin"
source = "lib://flowr/stdio/stdin.toml"
```

#### Inputs

#### Output
* Text read from STDIN - with leading and trailing whitespace (including EOF) trimmed.