# The Inspirations for 'flow'
I have had many sources of inspiration in this area over the past three decades.

Without realizing it they started to coalesce in my head and seemingly unrelated ideas from very different areas
started to come together to form what I eventually called 'flow' and I started to work on it.

The impetus to actually implement something, instead of just thinking about it, came when I was looking for some
"serious", more complex, project in order to learn  rust (and later adding WebAssembly to the mix).

It should be noted, that this project was undertaken in a very "personal" (i.e. idiosyncratic) way, without
any formal background in the area of functional programming, data flow programming, communicating serial
processes or similar. When starting it, I wanted to see if any of my intuitions and ideas could work, ignoring
previous efforts or established knowledge and previous projects. I didn't want to get "sucked in" to just 
re-implementing someone else's ideas.

I have done quite a bit of reading of paper on these areas after getting a reasonable version of `flow` working
and saw I was repeating a number of existing ideas and techniques..no surprise!

## Specific inspirations from my career
I have worked with these technologies listed below over the decares (from University until now) and they all added
something to the idea for flow in my head.

* The [Inmos transputer](https://en.wikipedia.org/wiki/Transputer) chip and its 
[Occam](https://en.wikipedia.org/wiki/Occam_(programming_language)) parallel programming
language (which I studied at University in the '80's), without realizing that this was based on Hoare's CSP.
  * Parallel programming language (although not based on data dependencies)
  * Parallel hardware 8and software processes) that communicated by sending messages over connections (some virtual in
  software, others over hardware between chips)
* [Structured Analysis and Design](https://en.wikipedia.org/wiki/Structured_analysis_and_design_technique) from 
my work with it in HP the '80s!
  * Hierarchical functional decomposition
  * Encapsulation
  * Separation of Program from Context
* UNIX pipes
  * Separate processes, each responsible for limited functionality, communicating in a pipeline via messages (text) 
* [Trace scheduling](https://en.wikipedia.org/wiki/Trace_scheduling) for compiler instruction scheduling based on data
dependencies between instructions (operations) work done at [MultiFlow](https://en.wikipedia.org/wiki/Multiflow) and
later HP by Josh Fisher, Paolo Faraboschi and others.
  * Exploiting inherent parallelism by identifying data dependencies between operations
* [Amoeba distributed OS](https://en.wikipedia.org/wiki/Amoeba_(operating_system)) by Andrew Tannenbaum that made a 
collaborating network of computers appear as one to the user of a "Workstation"
  * Distribution of tasks not requiring "IO", abstraction of what a machine is and how a computer program can run
* [Yahoo! Pipes](https://en.wikipedia.org/wiki/Yahoo!_Pipes) system for building "Web Mashups"
  * Visual assembly of a complex program from simpler process by connecting them together with data flows  
