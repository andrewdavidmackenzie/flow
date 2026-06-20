## Time Series Chart (//flowstdlib/charts/time_series)
Render an array of numeric values as a time series bar chart (Nx128 pixel grayscale).
Each element becomes a vertical bar, height proportional to the value within the
min-max range of the array. Auto-scales to fit the data.

Useful for visualizing how a value changes over time — connect to a sliding
window of recent values for a moving chart.

### Include using
```toml
[[process]]
source = "lib://flowstdlib/charts/time_series"
```
