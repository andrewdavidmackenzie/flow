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

## One output to multiple destinations (fan-out)

A single output can connect to multiple destinations simultaneously using array
syntax. The value is sent to all destinations in parallel:

```toml
flow = "two-destinations"

[[process]]
source = "context://args/get"

[[process]]
source = "context://stdio/stdout"

[[process]]
source = "context://file/file_write"
input.bytes = { once = [1, 2] }

# One output goes to BOTH stdout and file_write simultaneously
[[connection]]
from = "get/string/1"
to = ["stdout", "file_write/filename"]
```

**Feature**: Fan-out using `to = ["dest1", "dest2"]` array syntax. The value is
duplicated to all destinations. This is the basis for fan-out parallelism — one
data source feeding multiple independent processing paths.

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

## Reading command-line arguments

Read a specific argument by position using `context://args/get` with
`/string/N` (as text) or `/json/N` (as parsed JSON):

```toml
flow = "args"

[[process]]
source = "context://args/get"

[[process]]
source = "context://stdio/stdout"

# Read the 3rd argument as a string
[[connection]]
from = "get/string/3"
to = "stdout"
```

**Feature**: `args/get` provides command-line arguments. Use `/string/N` for
raw text or `/json/N` for parsed values (numbers, arrays, objects).

## Array accumulation, decomposition, and counting

Generate a stream of numbers, accumulate them into fixed-size arrays,
then process each element individually (auto-decomposition) and count them:

```toml
flow = "arrays"

# Generate numbers 10..29
[[process]]
source = "lib://flowstdlib/math/sequence"
input.start = { once = 10 }
input.limit = { once = 29 }
input.step = { once = 1 }

# Accumulate into arrays of size 4
[[process]]
alias = "accumulator"
source = "lib://flowstdlib/data/accumulate"
input.partial = { once = [] }
input.chunk_size = { always = 4 }

[[connection]]
from = "sequence/number"
to = "accumulator/values"

# Loop back partial array
[[connection]]
from = "accumulator/partial"
to = "accumulator/partial"

# Add 1 to every number — array chunks auto-decompose to individual values
[[process]]
source = "lib://flowstdlib/math/add"
input.i2 = { always = 1 }

[[connection]]
from = "accumulator/chunk"
to = "add/i1"

# Count the numbers produced
[[process]]
source = "lib://flowstdlib/data/count"
input.count = { once = 0 }

[[connection]]
from = "add"
to = "count/data"

[[connection]]
from = "count"
to = "count/count"

[[connection]]
from = "count"
to = "stdout"

[[process]]
source = "context://stdio/stdout"
```

**Features**:
- `accumulate` collects a stream of values into fixed-size arrays with
  loopback on `partial`
- Array auto-decomposition: connecting `accumulator/chunk` (array) to
  `add/i1` (number) automatically sends elements one by one
- `count` with loopback tracks how many values have been processed
- `always` initializer for constants (`chunk_size`, `i2`)

## Multi-stage pipeline

A linear chain of math operations, each feeding the next. Demonstrates
process aliasing and how data flows through a sequence of transformations:

```toml
flow = "pipeline"

[[process]]
source = "lib://flowstdlib/math/sequence"
input.start = { once = 1 }
input.limit = { once = 20 }
input.step = { once = 1 }

[[process]]
alias = "add1"
source = "lib://flowstdlib/math/add"
input.i2 = { always = 1 }

[[connection]]
from = "sequence/number"
to = "add1/i1"

[[process]]
alias = "divide2"
source = "lib://flowstdlib/math/divide"
input.divisor = { always = 2 }

[[connection]]
from = "add1"
to = "divide2/dividend"

[[connection]]
from = "divide2/result"
to = "stdout"

[[process]]
source = "context://stdio/stdout"
```

**Features**:
- Process aliasing (`alias = "add1"`) to use multiple instances of the
  same function with different names
- Linear pipeline: each function's output feeds the next function's input
- Named output selection (`divide2/result` vs `divide2/remainder`)

## Comparison, tap, and conditional output

Compare a computed value to a threshold and conditionally output using
`tap`. Demonstrates basic flow control:

```toml
flow = "primitives"

# 1 + 2 = 3
[[process]]
source = "lib://flowstdlib/math/add"
input.i1 = { once = 2 }
input.i2 = { once = 1 }

# 3 / 2 = 1.5
[[process]]
source = "lib://flowstdlib/math/divide"
input.divisor = { once = 2 }

[[connection]]
from = "add"
to = "divide/dividend"

# Compare 1.5 to 1
[[process]]
source = "lib://flowstdlib/math/compare"
input.right = { once = 1 }

[[connection]]
from = "divide/result"
to = "compare/left"

# Output the comparison result (gt = true since 1.5 > 1)
[[connection]]
from = "compare/gt"
to = "stdout"

# Tap blocks data when control is false
[[process]]
source = "lib://flowstdlib/control/tap"
input.control = { once = false }

[[connection]]
from = "divide/result"
to = "tap/data"

# Tap output is blocked (control=false), so nothing reaches stdout from here
[[connection]]
from = "tap"
to = "stdout"

[[process]]
source = "context://stdio/stdout"
```

**Features**:
- `compare` produces boolean outputs (`gt`, `lt`, `equal`, etc.)
- `tap` gates data flow based on a boolean control input
- Multiple inputs initialized with `once` for one-shot computation
