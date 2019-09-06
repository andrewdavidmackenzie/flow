## FormatBitmap (//flowstdlib/img/format_png)
Format a series of bytes into a PNG image, for use in display or writing to a file

#### Include using
```
[[process]]
alias = "format"
source = "lib://flowstdlib/img/format_png"
```

#### Input
* `bytes` - the bytes to encode as a bitmap
* `bounds` - Json with width and depth of the image the bytes represent

#### Output
* (default) - The bytes representing the PNG encoding of the image, as a String