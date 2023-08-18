## Contributing
There are many ways of contributing:
- adding an issue with a bug report of an enhancement request or new feature idea
- pick up an
[issue](https://github.com/users/andrewdavidmackenzie/projects/2/views/1) and try 
and fix it (I am not very good at labelling these, but will try)
- adding to or correcting the code documentation or the book
- adding a new example
- improvements to the libraries, compiler, standard library, run-time
- improvements to unit or integration tests
- improvements to build processes

To get started, fork the [repo](http://github.com/andrewdavidmackenzie/flow), 
clone it locally and build from source, as described in [building](building.md).

Maybe run an example or two. Samples that don't require specific arguments or standard 
input to work correctly (such as fibonacci) are the easiest to get started.

Then, once you know everything is working correctly, chose an issue to work on 
from the [GitHub project kanban](https://github.com/users/andrewdavidmackenzie/projects/2/views/1).

Create a branch to work on, and dive in. Try to make descriptive commit messages
of limited scope, and stick to the scope of the issue.

Make sure all code builds, there are no clippy errors, tests pass, and the books builds,
before pushing, by running `make`.

When you think you are done, you can create a Pull-Request to the upstream project.
If you include "Fixes #xyq" in the PR description, it will close the issue "xyz"
when it is merged.

If you are not sure if it is ready or want some early feedback, prefix the name of the 
PR with "WIP - " and I will know it's not intended to be merged yet.

If in doubt, just reach out to me by email to andrew@mackenzie-serres.net, create an
issue in GitHub, comment an existing issue or message me on matrix
(andrewdavidmackenzie:matrix.org).