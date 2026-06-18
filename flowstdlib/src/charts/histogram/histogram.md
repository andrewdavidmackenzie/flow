## Histogram Chart (//flowstdlib/charts/histogram)
Render an array of numbers as a bar chart image (Nx128 pixel grayscale).
Width matches the number of bins — pass any number of elements.
Each element becomes a vertical bar, height proportional to the maximum value.
Black bars on a white background.

Useful for visualizing frequency distributions, histograms, spectra, etc.

### Include using
```toml
[[process]]
source = "lib://flowstdlib/charts/histogram"
```
