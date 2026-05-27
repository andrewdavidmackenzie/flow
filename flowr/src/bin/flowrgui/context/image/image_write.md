image_write
==

Write an entire 2D grid to an image in one operation.

Each cell value maps to grayscale: 0 = white, non-zero = black.
The grid dimensions (rows × columns) determine the image size.

Inputs
===
* `grid` - A 2D array of numbers (`array/array/number`). Each row is an array of cell values.
* `filename` - The name identifier for the image buffer.
