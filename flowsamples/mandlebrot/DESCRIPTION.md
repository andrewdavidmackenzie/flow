mandlebrot
==
Render a mandlebrot set into an image file, with the output image size and imaginary number coordinate 
space configured via input parameters.

The pre-configured test (input arguments in [test.args](test.args)) renders a very small mandlebrot 
set (25x25 pixels) in order to keep the test running time time short and be able to use in in CI runs.

<a href="expected.file" target="_blank"><img src="expected.file"></a>

Description
===
Notably, there is also a standalone rust project in the `project` ([Cargo manifest](project/Cargo.toml)) folder.
The functions are used in the rust program that is built and also made available as functions to the 
Flow project that is described in the toml files - showing how native code can live alongside and be used by 
the flow.

Root Diagram
===
<a href="root.dot.svg" target="_blank"><img src="root.dot.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Root Flow
* subflow described separately, with named outputs to parent flow
* Connections between Input/Outputs of parent/child flows
* Setting initial value of a function with a `Once` initializer
* Multiple connections into and out of functions and sub-flows
* Library Functions used to convert Number to String and to add numbers
* Use of aliases to refer to functions with different names inside a flow
* Connections between flows, functions and values
* `flowr` `context function` used to render output to an [Image Buffer](../../flowr/src/cli/image/image_buffer.md)
* `provided functions` in rust that get compiled to WASM and then loaded and executed by the runtime

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
<a href="functions.dot.svg" target="_blank"><img src="functions.dot.svg"></a>

Click image to view functions graph.

SubFlows and Functions Description
===
- Subflow [parse_args](parse_args.toml) reads the argument passed to the flow and outputs the filename to render to, 
the size (width, height in `array/number`) and bounds of coordinate space (an array of 2 imaginary numbers,
where an imaginary number is two numbers, so expressed as `array/array/number`) to calculate the set for
- Subflow [generate pixels](generate_pixels.toml) that enumerates the 2D array of pixels to calculate, producing
"a stream" of pixels (x, y coordinates) to be used to calculate the appropriate value for that pixel.
- Subflow [render](render.toml) that uses the functions below to take the pixels, calculate it's location
in the 2D imaginary space, calculate the value in the set for that point and then render value at the pixel
in the image buffer.
  - Function [pixel to point](pixel_to_point/pixel_to_point.md) to calculate the corresponding location
in the 2D imaginary coordinate system for each pixel
  - Function [escapes](escapes/escapes.md) to calculate the appropriate value (using the core mandlebrot algorithm) for
  each pixel.