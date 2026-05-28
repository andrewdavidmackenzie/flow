## hamming_step

Extracts the minimum from a sorted array of candidate Hamming numbers,
then generates new candidates by multiplying the minimum by 2, 3, and 5.
The new candidates are merged into the remaining array, sorted, and
deduplicated.

This implements one step of Dijkstra's algorithm for generating Hamming
numbers (also called regular numbers or 5-smooth numbers).
