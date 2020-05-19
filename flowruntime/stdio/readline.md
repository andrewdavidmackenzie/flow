## Readline (//flowruntime/stdio/readline)
Read a line of text from the STDIN of the process invoking the flow. The line is terminated by EOL
but leading and trailing whitespace are trimmed before being output.

The function will be scheduled for running again, until EOF is detected, after which it will not run
again.

### Definition
```toml
{{#include readline.toml}}
```

### Include using
```toml
[[process]]
alias = "readline"
source = "lib://flowruntime/stdio/readline"
```

### Inputs

#### Output
* Line of text read from STDIN - with leading and trailing whitespace trimmed.