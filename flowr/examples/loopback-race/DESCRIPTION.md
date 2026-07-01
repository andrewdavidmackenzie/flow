loopback-race
==

Minimal reproducer for loopback-clearing race condition (#2887).

The `replicate` sub-flow should output a value N times using a
loopback pattern (multiply by 0, add value, loopback). If the
sub-flow goes idle between iterations and the runtime clears
internal inputs, the loopback value is destroyed and fewer
outputs are produced.
