## Zip (//flowstdlib/data/zip.toml)
Takes two arrays of values and produce an array of tuples of pairs of values from each input array.

#### Include using
```
[[process]]
alias = "zip"
source = "lib://flowstdlib/data/zip.toml"
```


#### Input
* left - the 'left' array
* right - the 'right' array

#### Outputs
* tuples - the array of tuples of (left, right)