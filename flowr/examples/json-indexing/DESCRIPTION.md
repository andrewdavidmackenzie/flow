json-indexing
==

Description
===
A flow that demonstrates how to index into a (Json) Value (the 'json' output of 'readline') that is 
happens to be an array of values at run-time. 

Features Used
===
* Root Flow
* Library Functions used (`readline` and `stdout` from `context`)
* Connections between functions
* Connection from a specific array element of a Json output value (from readline/json) using:
```
[[connection]]
from = "readline/json/2"
to = "stdout"
```