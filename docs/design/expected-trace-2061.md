# Expected Trace for sequence-of-sequences (#2061)

With `--jobs 1 -v debug`. With right-lt fix + step-gate.

## Functions (with step-gate)
```
F#0: add           flow=1 (outer)  i1, i2(flow_init once:1)
F#1: compare_switch flow=1 (outer)  left(flow_init once:9), right(flow_init once:1)
F#2: step-gate     flow=1 (outer)  data, control
F#3: add           flow=2 (inner)  i1, i2
F#4: compare_switch flow=2 (inner)  left, right(flow_init always:0)
F#5: step-gate     flow=2 (inner)  data, control
F#6: stdout        flow=0 (root)   generic
```

## INIT
```
Initialized F#0:1 in flow 1 (queue depth: 1)     once:1
Initialized F#1:0 in flow 1 (queue depth: 1)     once:9
Initialized F#1:1 in flow 1 (queue depth: 1)     once:1
Initialized F#4:1 in flow 2 (queue depth: 1)     always:0
F#1 Ready [9, 1] → Job #1. Flow 1 Busy.
```

## Job #1: F#1 compare(f1) [9,1]
```
9 > 1 → right-lte:1, right-lt:1, left-gt:9
  /right-lte=1 → F#3:1  cross-flow  F#3:1 depth: 1   step for inner add
  /right-lte=1 → F#4:0  cross-flow  F#4:0 depth: 1   limit for inner compare
    F#4 Ready [1, 0] → Job #2. Flow 2 Busy.
  /left-gt=9   → F#1:0  loopback    F#1:0 depth: 1   feedback-limit
  /right-lt=1  → F#0:0  same-flow   F#0:0 depth: 1   previous for outer add
    F#0 Ready [1, 1] → Job #3.
  /right-lt=1  → F#2:1  same-flow   F#2:1 depth: 1   outer step-gate control
Job #1: busy_flows={2: [4], 1: [0]} blocked={}
Job #1: F#1(1) input queues: [1, 0]
Job #1: F#2(1) input queues: [0, 1]
```

## Job #2: F#4 compare(f2) [1,0]
```
1 > 0 → right-lte:0, right-lt:0, left-gt:1
  /right-lte=0 → F#6:0  cross-flow  STDOUT "0". Job #4. Flow 0 Busy.
  /left-gt=1   → F#4:0  loopback    feedback-limit
  /right-lt=0  → F#3:0  same-flow   previous for inner add
    F#3 Ready [0, 1] → Job #5.
  /right-lt=0  → F#5:1  same-flow   inner step-gate control
Job #2: busy_flows={2: [3], 1: [0], 0: [6]} blocked={}
Job #2: F#4(2) input queues: [1, 0]
Job #2: F#5(2) input queues: [0, 1]
```

## Job #3: F#0 add(f1) [1,1] → 2
```
  i2=1 → F#2:0  (step-gate data, NOT loopback)  F#2:0 depth: 1
    F#2 Ready [1, 1] → Job #6.
  output=2 → F#1:1  same-flow  F#1:1 depth: 1
Job #3: busy_flows={2: [3], 1: [2], 0: [6]} blocked={1}
Job #3: F#1(1) input queues: [1, 1]
```

## Job #4: F#6 stdout(f0) [0]
```
  STDOUT prints "0". Flow 0 Idle.
Job #4: busy_flows={2: [3], 1: [2]} blocked={1}
```

## Job #5: F#3 add(f2) [0,1] → 1
```
  i2=1 → F#5:0  (step-gate data)  F#5:0 depth: 1
    F#5 Ready [1, 0] → Job #7.
  output=1 → F#4:1  same-flow  F#4:1 depth: 1
    F#4 Ready [1, 1] → Job #8. Flow 2 Busy.
Job #5: busy_flows={2: [5, 4], 1: [2]} blocked={1}
```

## Job #6: F#2 step-gate(f1) [1, 1] → 1
```
  output=1 → F#0:1  same-flow  F#0:1 depth: 1
  Flow 1 Idle.
Job #6: busy_flows={2: [5, 4]} blocked={1}
Job #6: F#0(1) input queues: [0, 1]
Job #6: F#1(1) input queues: [1, 1]
```

## Job #7: F#5 step-gate(f2) [1, 0] → 1
```
  output=1 → F#3:1  same-flow  F#3:1 depth: 1
Job #7: busy_flows={2: [4]} blocked={1}
Job #7: F#3(2) input queues: [0, 1]        ← step=1 delivered to add:i2
```

## Job #8: F#4 compare(f2) [1,1] — equal
```
1 == 1 → equal → right-lte:1 (NO left-gt, NO right-lt)
  /right-lte=1 → F#6:0  cross-flow  STDOUT "1". Job #9.
  (no right-lt → no send to add:i1 or step-gate:control)
  Flow 2 Idle.
    Unblocks F#1 → Ready [9, 2] → Job #10. Flow 1 Busy.
    Flow initializers: always:0 on F#4:1 → F#4:1 depth: 1

>>> DIVERGENCE <<<

EXPECTED: F#3(2) input queues: [0, 0]       add:i2 empty
ACTUAL:   F#3(2) input queues: [0, 1]       add:i2 has stale step=1

The step-gate fired in Job #7 and delivered step=1 to F#3:i2.
But F#4 then output equal (Job #8) — no right-lt — so F#3:i1 never filled.
The step on F#3:i2 is stale — same problem as the direct loopback,
just delivered through step-gate one cycle ahead.

Job #8: busy_flows={1: [1], 0: [6]} blocked={}
Job #8: F#3(2) input queues: [0, 1]         ← stale step
Job #8: F#4(2) input queues: [0, 1]         ← fresh always:0
Job #8: F#5(2) input queues: [0, 0]         ← step-gate clean ✓
```

## Summary

The step-gate prevents stale values in step-gate itself (F#5 is clean).
But it still delivers the step to add:i2 one cycle ahead of when add uses it.
On the last cycle, step-gate fires (it had both data and control from the
keep-going cycle), sending step to add:i2. Then compare outputs equal —
add:i1 never fills — and the step sits stale on add:i2.

The fundamental timing: step-gate control (right-lt) fires at compare-time,
not at "next compare" time. So step-gate fires one cycle before we know
whether add will run again.
