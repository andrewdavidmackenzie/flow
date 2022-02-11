## Split (//flowstdlib/data/split)
Split a string into (possibly) two parts and a possible token, based on a separator.

This function is implemented in a deliberate way to be able to showcase parallelization.

Instead of going through the string in order looking for the separator and gathering an array
of sections it takes an alternative approach.

It starts in the middle of the string looking for a separator character from there towards the
end. If it finds one then the string is split in two and those two sub-strings are output as
an array of strings on the `partial` output. NOTE that either or both of these two sub-strings
may have separators within them, and hence need further subdivision.

For that reason, the `partial` output is feedback to the `string` input, and the runtime will
serialize the array of strings to the input as separate strings.

If from the middle to the end no separator is found, then it tries from the middle backwards
towards the beginning. If a separator is found, the two sub-strings are output on `partial`
output as before.

If no separator is found in either of those cases, then the string doesn't have any and is
output on the `token` output.

Thus, strings with separators are subdivided until strings without separators are found, and
each of those is output as a token.

Due to the splitting and recursion approach, the order of the output tokens is not the order
they appear in the string.

### Definition
```toml
{{#include split.toml}}
```

### Include using
```toml
[[process]]
source = "lib://flowstdlib/data/split"
```