# Parallelism
Using `flow` algorithms can be defined that exploit multiple types of parallelism:
- Data Parallelism
- Pipelining
- Divide and Conquer

## Data Parallelism
Also known as "Single Program Multiple Data".

In this case the data is such that it can be segmented and worked on in parallel, using the same basic algorithm
for each chunk of data.

An example would be some image processing or image generation task, such as generating the mandlebrot set
(see the [mandlebrot](../../flowsamples/mandlebrot/DESCRIPTION.md) example in `flowsamples`).

The two-dimensional space is broken up into a 2D Array of pixels or points, and then they are streamed through
a function or sub-flow that does some processing or calculation, producing a new 2D Array of output values.

Due to the data-independence between them, all of them can be calculated/processed in parallel, across many
threads or even machines, exploiting the inherent parallelism of this "embarrassingly parallel" algorithm.

They need to be combined in some way to produce the meaningful output. This could be using an additional sub-flow
to combine them (e.g. produce an average intensity or color of an image), that is *not* parallel, or it 
could be to render them as an image for the user. 

In the case of producing a file or image for the user, functions can be used for that from
the flow runner's `context functions` leaving the flow itself totally parallel.

In a normal procedural language, an image would be rendered in memory in a 2D block of pixels and then 
written out to file sequentially so that the pixels are placed in the correct order/location in the file.

In a flow program, that could be gone, although accumulating the 2D array in memory may represent a bottleneck.
`flowr`'s [image buffer](../../flowr/src/cli/image/image_buffer.md) `context function` is written such that it can 
accept pixels in any random order and render them correctly, but having the following inputs:
```
### Inputs
* `pixel` - the (x, y) coordinate of the pixel
* `value` - the (r, g, b) triplet to write to the pixel
* `size`  - the (width, height) of the image buffer
* `filename` - the file name to persist the buffer to
```

## Map Reduce
Map-Reduce is done similar to above, using a more complex initial step to form independent data "chunks"
("Mapping") that can be processed totally in parallel, and a combining phase ("Reducing) to produce the 
final output.

## Pipelining
A `flow` program to implement pipeline processing of data is trivial and there is a 
[pipeline](../../flowsamples/pipeline/DESCRIPTION.md) example in`flowsamples`.

A series of processes (they can be `functions` or `subflows`) are defined. Input data is connected to flow
to the first, whose output is sent to the second, and so on and the output rendered for the user.

When multiple data values are sent in in short succession (additional values are sent before the first value
has propagated out of the output) then multiple of the processes can run in parallel, each one operating on
a different data value, as there is no data or processing dependency between the data values.

If there are enough values (per unit time) to demand it, multiple instances of the same processes can be used
to increase parallelism, doing the same operation multiple times in parallel on different data values.

## Divide and Conquer
Just as in procedural programming, a large problem can be broken down into separate pieces and 
programmed separately, this can be done with `flow`.

A complex problem could be broken down into two (say) largely independent sub-problems. Each one can 
be programmed in different sub-flows, and fed different parts (or copies) of the input data. Then when 
both produce output they can be combined in some way for the user.

As there is no data dependency between the sub-flow outputs (intermediate values in the grander scheme of
things) they can run totally in parallel. 

If the two values were just need to be output to the user, then they can each proceed at their own pace
(in parallel) and each one output when complete. In this case the order of the values in the output to the
user might vary, and appropriate labelling to understand them will be needed.

Depending on how the values need to be combined, or if a strict order in the output is required,
then a later ordering or combining step maybe needed. This step will necessarily depends on both sub-flow's
output value, thus introducing a data dependency and this final step will operate without parallelism.

Providing the final (non-parallel step) is less compute intensive than the earlier steps, an overall 
gain can be made by dividing and conquering (and then combining).

