# Render Pixel

// TODO rewrite as this is wrong

Given the row and column of a pixel in the output image, return the corresponding point on the complex plane.

* `size` is a pair giving the width and height of the image in pixels.
* `pixel` is a (row, column) pair indicating a particular pixel in that image.
* `bounds` is two complex numbers - `upper_left` and `lower_right` designating the area our image covers.

Try to determine if 'c' is in the Mandelbrot set, using at most 'limit' iterations to decide if 'c' is not a member, 
return 'Some(i)', where 'i' is the number of iterations it took for 'c' to leave the circle of radius two centered on the origin.

If 'c' seems to be a member (more precisely, if we reached the iteration limit without being able to prove that 'c' 
is not a member) return 'None'