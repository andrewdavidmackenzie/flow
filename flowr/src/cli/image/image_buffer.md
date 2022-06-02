## ImageBuffer (//context/image/image_buffer)
Write `pixels` to an image buffer

### Definition
```toml
{{#include image_buffer.toml}}
```

### Include using
```toml
[[process]]
alias = "buffer"
source = "context://file/image_buffer"
```

### Inputs
* `pixel` - the (x, y) of the pixel
* `value` - the (r, g, b) triplet to write to the pixel
* `size`  - the (width, height) of the image buffer
* `filename` - the file name to persist the buffer to