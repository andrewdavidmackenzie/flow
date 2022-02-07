# Escapes

Try to determine if 'c' is in the Mandelbrot set, using at most 'limit' iterations to decide if 'c' is not a member, 
return 'Some(i)', where 'i' is the number of iterations it took for 'c' to leave the circle of radius two centered on the origin.

If 'c' seems to be a member (more precisely, if we reached the iteration limit without being able to prove that 'c' 
is not a member) return 'None'