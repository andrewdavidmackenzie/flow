## Bin Count (//flowstdlib/data/bin_count)
Count occurrences of each value in a stream. Each input value is used as
an index into a bins array, incrementing the count at that position.

On null input (EOF), outputs the final bins array on the `bins` output.
On non-null input, outputs updated partial bins on `partial` for loopback.

Useful for histograms, frequency analysis, categorical counting.

### Include using
```toml
[[process]]
source = "lib://flowstdlib/data/bin_count"
```
