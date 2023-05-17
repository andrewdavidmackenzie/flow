## Readline (//context/stdio/readline)
Read a line of text from the STDIN of the process invoking the flow. The line is terminated by EOL
but leading and trailing whitespace are trimmed before being output.

The function will be scheduled for running again, until EOF is detected, after which it will not run
again.

### Include using
```toml
[[process]]
alias = "readline"
source = "context://stdio/readline"
```

### Inputs
* prompty - String prompt, or "" (empty string) can be used for none.


#### Output
* text - Line of text read from STDIN - with leading and trailing whitespace trimmed.
* json - Json value parsed from from STDIN