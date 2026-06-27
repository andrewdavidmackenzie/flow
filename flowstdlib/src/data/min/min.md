## Stream Min (//flowstdlib/data/min)
Track the minimum value in a null-terminated stream of numbers.

Feed values one at a time. When a null is received, the final minimum
is output on `result`.

### Inputs
* `value` (number) — the next value in the stream (null to finish)
* `partial` (number) — current minimum (initialize to a large value, feed back from output)

### Outputs
* `partial` (number) — running minimum (feed back to input)
* `result` (number) — final minimum (emitted on null)

### Include using
```toml
[[process]]
source = "lib://flowstdlib/data/min"
```
