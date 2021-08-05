## Status
The semantics of flows, processes, connections etc along with the implementation of the `flowc` compiler, `flowr` 
runner, `flowruntime` code runtime functions and the `flowstdlib` library of functions and flows has allowed for the 
creation of a set of example flows that execute as expected.

There is pretty good overall test coverage (> 82%) that allows for safer refactoring.

The docs are reasonably extensive but a bit out of date in places, but I have issues created to cover most of the most
significant docs improvements needed.

The project continues to evolve in [the flow repo][https://github.com/andrewdavidmackenzie] with myself as the 
only contributor.

I moved some GUI/IDE experimentation into a separate repo that uses the `flowclib`  and `flowrlib` libs from here,
and that is the biggest area needing work in order to showcase `flow` and make it more visual and easy to follow
for folks.