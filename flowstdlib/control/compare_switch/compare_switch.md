## CompareSwitch (//flowstdlib/control/compare_switch)
### Description
Compares two input values and outputs the right hand and left hand values on different outputs, 
depending on the comparison result:

#### **equal**
The left/right value is output on the "equal" output
#### **greater than**
The left value is output on the "left-gt", right value on the "right-gt" output
#### **greater than or equal**
The left value is output on the "left-gte", right value on the "right-gte" output
#### **less than**
The left value is output on the "left-lt" output, right value is output on the "right-lt" output
####**less than or equal**
The left value is output on the "left-lte" output, right value is output on the "right-lte" output

### Usage
```toml
[[process]]
source = "lib://flowstdlib/control/compare_switch"
```

### Definition
```toml
{{#include compare_switch.toml}}
```