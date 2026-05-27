game_step
==

Computes the next generation of Conway's Game of Life.

Inputs
===
* `grid` - Either a seed pattern name (string, e.g. "glider") for initialization,
  or a flat array of 0/1 values representing the current grid state.
* `size` - Array `[width, height]` specifying the grid dimensions.

Outputs
===
* `grid` - The next generation grid as a flat array of 0/1 values.
* `pixels` - Array of `[[x,y],[r,g,b]]` pairs for rendering. Alive cells
  are black `[0,0,0]`, dead cells are white `[255,255,255]`.

Rules
===
Conway's Game of Life rules with wrapping boundaries:
* A live cell with 2 or 3 neighbors survives
* A dead cell with exactly 3 neighbors becomes alive
* All other cells die or stay dead
