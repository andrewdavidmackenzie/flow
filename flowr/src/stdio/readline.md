## Readline (//flowr/stdio/readline.toml)
Read a line of text from the STDIN of the process invoking the flow. The line is terminated by EOL
but leading and trailing whitespace are trimmed before being output.

The function will be scheduled for running again, until EOF is detected, after which it will not run
again.

#### Include using
```
[[process]]
alias = "readline"
source = "lib://flowr/stdio/readline.toml"
```

#### Inputs

#### Output
* Line of text read from STDIN - with leading and trailing whitespace trimmed.