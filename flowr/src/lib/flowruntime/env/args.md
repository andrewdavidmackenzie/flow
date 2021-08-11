## Args (//flowruntime/env/args)
Get the arguments the flow was executed with

### Definition
```toml
{{#include args.toml}}
```

### Reference using
```toml
[[connection]]
from = "flowruntime/env/args"
to = "flowruntime/stdio/stdout"
```

### Inputs

### Output
* text - Array of Strings of the command line arguments the flow was invoked with.
* json - Array of Json parsed values of the command line arguments the flow was invoked with.