## Stream Avg (//flowstdlib/data/avg)
Compute the running average of a null-terminated stream of numbers.

Feed values one at a time. When a null is received, the final average
is output on `result` and the count of values on `count`.

### Inputs
* `value` (number) — the next value in the stream (null to finish)
* `partial_sum` (number) — accumulated sum (initialize to 0, feed back from output)
* `partial_count` (number) — accumulated count (initialize to 0, feed back from output)

### Outputs
* `partial_sum` (number) — running sum (feed back to input)
* `partial_count` (number) — running count (feed back to input)
* `result` (number) — final average (emitted on null)
* `count` (number) — total number of values (emitted on null)

### Include using
```toml
[[process]]
source = "lib://flowstdlib/data/avg"
```
