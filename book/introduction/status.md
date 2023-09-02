## Status
The semantics of flows, processes and connections along with the implementation of the `flowc` compiler, `flowrcli` 
runner, `context` functions and the `flowstdlib` library has allowed for the 
creation of a set of example flows that execute as expected.

There has pretty good overall test coverage (> 84%) that allows for safer refactoring.

The book is reasonably extensive but can always be improved. They probably need "real users" (not the author)
to try to use them and flow to make the next round of improvements. There are issues in the repo
and the [project](https://github.com/users/andrewdavidmackenzie/projects/2/views/1) related to improving the book.

I have added an experimental GUI for running flows in `flowrgui` that uses the rust Iced GUI toolkit.