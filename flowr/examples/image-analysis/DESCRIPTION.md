Image Analysis and Enhancement
==

Description
===
Demonstrates two levels of parallelism in a real-world image processing
pipeline:

1. **Fan-out parallelism**: The histogram's derived statistics (min, max,
   average brightness, pixel count) are computed simultaneously from the
   histogram data and output to stdout independently.

2. **Data parallelism**: The contrast stretch operation remaps every pixel
   independently using min/max from the histogram, then writes the
   enhanced image.

The flow reads a grayscale image, computes its histogram and statistics,
applies contrast stretching to enhance the image, and writes the result.

```
image_read → pixels
  ├─→ histogram → min ─┬─→ stdout (statistics, fan-out parallel)
  │                max ─┤
  │            average ─┤
  │              count ─┘
  │                │
  │          min, max → contrast_stretch (data parallel)
  │                          │
  └── pixels ────────────────┘
                             └─→ image_write (enhanced output)
```

Root Diagram
===
<a href="root.dot.svg" target="_blank"><img src="root.dot.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Provided functions (histogram, contrast_stretch — compiled to WASM)
* Context Functions
    * `args/get` for input/output filenames
    * `image_read` to read a PNG image as grayscale pixels
    * `image_write` to write the enhanced image
    * `stdout` to output statistics
* Library Functions (`to_string` for formatting)
* Fan-out: histogram outputs feed multiple independent paths simultaneously
* Data parallelism: contrast stretch processes all pixels independently
