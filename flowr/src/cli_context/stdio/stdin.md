## Stdin (//context/stdio/stdin)
Read text from the STDIN of the process invoking the flow until EOF is detected, after which it will not run
again. If you wish to get the value of a line (i.e. after ENTER is pressed, then use [readline](readline.md))

### Definition
```toml
{{#include stdin.toml}}
```

### Include using
```toml
[[process]]
alias = "stdin"
source = "context://stdio/stdin"
```

### Inputs

### Output
* text - Text read from STDIN - with leading and trailing whitespace (including EOF) trimmed.
* json - Json value parsed from from STDIN