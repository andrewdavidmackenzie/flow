# Feature Examples

This section contains small, focused flow definitions that each demonstrate a single
flow feature. They are useful as quick references when building your own flows.

## Reading JSON arguments

Access command-line arguments as JSON values using the `context://args/get` function
with `/json/N` output indexing:

```toml
flow = "args-json"

[[process]]
source = "context://args/get"

[[process]]
source = "context://stdio/stdout"

# Read the 3rd argument as a parsed JSON value
[[connection]]
from = "get/json/3"
to = "stdout"
```

**Feature**: Output route indexing (`/json/3`) to select the output type and position.
Compare with `/string/3` which returns the raw string instead of a parsed JSON value.

## Multiple connections to a single input

Multiple connections can feed the same input. Values arrive in the order they are
produced and queue up:

```toml
flow = "double-connection"

[[process]]
source = "context://args/get"

[[process]]
source = "context://stdio/stdout"

# Two different outputs both connect to the same stdout input
[[connection]]
from = "get/string/1"
to = "stdout"

[[connection]]
from = "get/string/3"
to = "stdout"
```

**Feature**: Multiple connections to one input — values queue up and are processed
in order.

## Reading JSON from stdin

Read and parse JSON values interactively from stdin using `context://stdio/readline`
with `/json` output indexing:

```toml
flow = "json-indexing"

[[process]]
source = "context://stdio/readline"
input.prompt = { always = "" }

[[process]]
source = "context://stdio/stdout"

# Parse each line as JSON and extract the 2nd element
[[connection]]
from = "readline/json/2"
to = "stdout"
```

**Feature**: `readline/json/N` parses stdin input as JSON and indexes into arrays.
Input `[1, 2, 3]` with `/json/2` outputs `3` (0-indexed).

## Echoing stdin with a prompt

Read lines from stdin and echo them back, with a configurable prompt using the
`always` initializer:

```toml
flow = "line-echo"

[[process]]
source = "context://stdio/readline"
input.prompt = { always = "> " }

[[process]]
source = "context://stdio/stdout"

[[connection]]
from = "readline/string"
to = "stdout"
```

**Feature**: The `always` initializer provides a constant value on every invocation.
Here it sets the prompt string that is displayed before each line of input.

## Generating a sequence with end detection

Generate a numeric sequence using `flowstdlib/math/sequence` and use `join` to
output a message when the sequence completes:

```toml
flow = "sequence"

[[process]]
source = "lib://flowstdlib/math/sequence"
input.start = { once = 2 }
input.step = { once = 3 }
input.limit = { once = 100 }

[[process]]
source = "context://stdio/stdout"

[[connection]]
from = "sequence/number"
to = "stdout"

# Output a string when the sequence ends
[[process]]
source = "lib://flowstdlib/control/join"
input.data = { once = "Sequence done" }

# The "last" output triggers join when the final number is produced
[[connection]]
from = "sequence/last"
to = "join/control"

[[connection]]
from = "join"
to = "stdout"
```

**Features**:
- The `once` initializer provides a value only on the first invocation
- The `sequence` library flow generates numbers from `start` to `limit` by `step`
- The `join` control function holds a value until a control signal arrives
- Multiple named outputs (`/number` and `/last`) from a single process
