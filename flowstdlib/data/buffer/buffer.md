## Buffer (//flowstdlib/data/buffer)
Takes a value on it's input and sends the same value on it's output when it can
run, effectively buffering it until the downstream processs can accept it.

#### Include using
```
[[process]]
alias = "buffer"
source = "lib://flowstdlib/data/buffer"
```


#### Input
* (default) - the value to buffer

#### Outputs
* the buffered value