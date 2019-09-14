## Zip (//flowstdlib/data/zip)
Takes two arrays of values and produce an array of tuples of pairs of values from each input array.

#### Include using
```toml
[[process]]
alias = "zip"
source = "lib://flowstdlib/data/zip"
```


#### Input
* left - the 'left' array
* right - the 'right' array

#### Outputs
* tuples - the array of tuples of (left, right)