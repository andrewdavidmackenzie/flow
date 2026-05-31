Hamming Numbers
==

Description
===
Generates [Hamming numbers](https://en.wikipedia.org/wiki/Regular_number)
(also called regular numbers or 5-smooth numbers) in ascending order.

A Hamming number is a positive integer whose only prime factors are 2, 3,
and 5. The sequence begins: 1, 2, 3, 4, 5, 6, 8, 9, 10, 12, 15, 16, 18, 20, ...

This is a classic problem in computer science, discussed by Dijkstra (1976)
and featured in "Lucid, the Dataflow Programming Language" (Wadge & Ashcroft,
1985, p.84) as a canonical dataflow example. The Lucid solution is:

```
h = 1 fby merge(merge(2*h, 3*h), 5*h)
```

The idea: if `h` is the stream of Hamming numbers, then `2*h`, `3*h`, and
`5*h` are all substreams of `h`. The complete stream is 1 followed by the
sorted merge (without duplicates) of these three substreams. This is
inherently self-referential — the stream is defined in terms of itself via
feedback.

The algorithm maintains a sorted set of candidate numbers. Each iteration
extracts the smallest candidate (the next Hamming number), then generates
three new candidates by multiplying it by 2, 3, and 5. Duplicates are
removed to avoid redundant work (e.g., 6 = 2×3 = 3×2).

Root Diagram
===
<a href="root.svg" target="_blank"><img src="root.svg"></a>

Click image to navigate flow hierarchy.

Functions Diagram
===
<a href="functions.svg" target="_blank"><img src="functions.svg"></a>

Click image to view functions graph.

Features Used
===
* Provided function (hamming_step - compiled to WASM)
* Library Functions used
    * `subtract` for iteration counting
    * `compare` for termination check
    * `tap` for conditional loopback
* Context Functions (`args/get`, `stdio/stdout`)
* Loopback connections for iterative generation
* Input initializers (`once` for initial candidates `[1]`)
