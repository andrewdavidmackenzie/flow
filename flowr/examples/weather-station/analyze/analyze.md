# analyze

Analyzes an array of temperature readings from a weather station sensor.

## Inputs
- `readings`: array of numbers (temperature values in °C)

## Outputs
- `report`: string with formatted analysis including:
  - Summary statistics (min, max, mean)
  - Time-of-day temperature profile
  - Detected anomalies (sudden changes > 3°C between consecutive readings)
- `bins`: array of 256 numbers for histogram visualization — temperature
  distribution mapped to 0-255 range for the histogram chart function
