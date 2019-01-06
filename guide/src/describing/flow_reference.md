## Flow Reference
A reference to a flow defined elsewhere:
* `alias` - a String that is used for display and referencing purposes within the flow it is used in.
* `ource` - the location where the flow is defined.

### Flow aliasing
When nesting the same subflow multiple times in a parent flow, there maybe a need to reference each one 
separately (e.g. to build connections to one or the other). For this reason a reference to a flow can contain 
an "alias" to re-name that included flow, and hence be able to reference it uniquely within the current flow
being defined.