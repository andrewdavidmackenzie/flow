# Trace Comparison: Expected vs Actual (#2061)

With right-lt fix + step-gate. Runtime unchanged from master.
Run with `--jobs 1 -v debug`. Vocabulary from flow-vocabulary.md.

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

## Jobs #1–#7: SAME

Both traces agree through Job #7. Step-gate fires correctly during
the inner iteration, delivering step to add:i2. See expected trace
for full details.

## Job #8: F#4 compare(f2) [1,1] — DIVERGENCE

Execution is the same (equal case, right-lte=1 to stdout, no right-lt).
Flow 2 goes Idle. Flow initializer fires on F#4:1 (always:0, depth: 1).

```
                           EXPECTED                    ACTUAL
Job #8: busy_flows         {1: [1], 0: [6]}            {1: [1], 0: [6]}        SAME
Job #8: blocked            {}                          {}                      SAME
Job #8: F#3(2) queues      [0, 0]                      [0, 1]                  DIFFERENT
Job #8: F#4(2) queues      [0, 1]                      [0, 1]                  SAME
Job #8: F#5(2) queues      [0, 0]                      [0, 0]                  SAME
```

F#3:i2 (inner add's step input) has stale value 1 from step-gate
output in Job #7. Step-gate itself is clean (F#5 queues [0, 0]).
But step-gate delivered the step to add:i2 one cycle ahead.

## Root Cause (same as before, step-gate doesn't help)

Step-gate is controlled by right-lt, which fires at compare-time.
Step-gate fires during the keep-going cycle and delivers step to
add:i2 for the NEXT cycle. On the last keep-going cycle, step-gate
fires and delivers step — but the next compare outputs equal (no
right-lt), so add:i1 never fills and the step is stale on add:i2.

The step-gate correctly prevents stale values in its OWN inputs.
But it delivers the step to add:i2 before we know if add will run
again. The stale value problem moved from "on add:i2 via direct
loopback" to "on add:i2 via step-gate", but the effect is identical.
