# sliding_window

Appends a new value to the window array. If the window exceeds 24 elements
(6 hours of readings at 15-min intervals), the oldest value is removed.

## Inputs
- `value`: a number (new temperature reading)
- `window`: array of numbers (current window, fed back from output)

## Outputs
- `window`: array of numbers (updated sliding window)
