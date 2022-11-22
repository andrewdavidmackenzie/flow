## Status
The semantics of flows, processes and connections along with the implementation of the `flowc` compiler, `flowr` 
runner, `context` functions and the `flowstdlib` library has allowed for the 
creation of a set of example flows that execute as expected.

There has pretty good overall test coverage (> 82%) that allows for safer refactoring.

The docs are reasonably extensive but can always be improved. They probably need "real users" (not the author)
to try to use them and flow to make the next round of improvements. There are issues in the repo
and the [project](https://github.com/users/andrewdavidmackenzie/projects/2/views/1) related to improving docs.

I moved some GUI/IDE experimentation into a separate repo that uses the `flowclib`  and `flowrlib` libs.
The intention is to re-start some GUI experimentation with `egui` or `iced` rust GUI libraries when 1.0
is released.