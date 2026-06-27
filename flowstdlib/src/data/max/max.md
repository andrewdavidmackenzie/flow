## Stream Max (//flowstdlib/data/max)
Track the maximum value in a null-terminated stream of numbers.

Feed values one at a time. When a null is received, the final maximum
is output on `result`.

### Inputs
* `value` (number) — the next value in the stream (null to finish)
* `partial` (number) — current maximum (initialize to 0, feed back from output)

### Outputs
* `partial` (number) — running maximum (feed back to input)
* `result` (number) — final maximum (emitted on null)

### Include using
```toml
[[process]]
source = "lib://flowstdlib/data/max"
```
