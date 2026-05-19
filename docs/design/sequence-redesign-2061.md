# Sequence Flow Redesign for Nested Loops (#2061)

## Problem

When the sequence sub-flow is used nested (sequence-of-sequences), stale loopback values
persist across flow idle boundaries, corrupting the next iteration. Two sources of stale values:

1. **add/i2** (step self-loopback `add/i2 → add/i2`) — the step value loops back after each add
   execution. When the sequence ends, the last looped step stays on add/i2.

2. **compare_switch/right** (add output) — on the equal case (limit == current), add runs one
   more time producing a value that exceeds the limit. This exceeded value goes to
   compare_switch/right, but compare_switch can't consume it (no left — left-gt doesn't fire
   when limit is not > current). The stale exceeded value sits on compare_switch/right.

## Solution: compare + dual tap gated feedback

Replace the unconditional `add/i2 → add/i2` self-loopback AND the direct `add → compare_switch/right`
connection with gated feedback paths using three new functions in the sequence flow:

1. **compare** (`lib://flowstdlib/math/compare`) — fed the same left (limit) and right (current)
   inputs as compare_switch. Outputs boolean `gt` which is `true` when iterating (left > right)
   and `false` on the equal or exceeded case.

2. **step-tap** (`lib://flowstdlib/control/tap`) — gates the step feedback. Takes add's consumed
   i2 as data and compare's `gt` as control. When `gt=true`, passes step through to add:i2.
   When `gt=false`, discards the step.

3. **output-tap** (`lib://flowstdlib/control/tap`) — gates add's output. Takes add's output as
   data and compare's `gt` as control. When `gt=true`, passes the next value through to
   compare_switch/right and compare/right. When `gt=false`, discards the exceeded value.

### Why this works

On the equal case (limit == current):
- compare outputs `gt=false` to both taps
- add runs (fed by right-lte → add/i1 and step from step-tap) and produces exceeded value
- output-tap receives (exceeded, false) → **discards** → compare_switch/right stays clean
- step-tap receives (step, false) → **discards** → add/i2 stays clean
- Flow goes idle with no stale values on any input

### Limitation: non-equal termination

This approach fully cleans stale values when the sequence terminates via the equal case.
When step doesn't divide evenly into (limit - start), the sequence terminates via exceeded
without passing through equal, leaving stale values on add/i2 and tap controls.

This is acceptable for sequence-of-sequences because the outer sends the same value as both
step and limit, so the inner always terminates on equal.

## Sequence flow structure

```text
Functions: compare_switch, compare, add, step-tap, output-tap

Connections:
  input/start   → compare_switch/right, compare/right  (first value)
  input/limit   → compare_switch/left, compare/left    (limit)
  input/step    → add/i2                                (initial step)

  compare_switch/right-lte → output/number              (stdout)
  compare_switch/right-lte → add/i1                     (previous value for add)
  compare_switch/left-gt   → compare_switch/left        (limit loopback)
  compare_switch/left-gt   → compare/left               (limit loopback to compare)
  compare_switch/right-gte → output/last                (sequence done signal)

  compare/gt               → step-tap/control           (boolean gate for step)
  compare/gt               → output-tap/control         (boolean gate for output)

  add/i2                   → step-tap/data              (consumed step to tap)
  step-tap output          → add/i2                     (gated step feedback)

  add output               → output-tap/data            (add result to tap)
  output-tap output        → compare_switch/right       (gated value feedback)
  output-tap output        → compare/right              (gated value feedback)
```

## Trace analysis

For inner sequence(0, 1, 1) — limit=1, step=1, start=0:

```text
Initial state:
  compare_switch: left=[1], right=[0]
  compare:        left=[1], right=[0]
  add:            i1=[], i2=[1]

Cycle 1 (keep going, limit=1 > current=0):
  compare(1, 0) → gt=true → step-tap/control, output-tap/control
  compare_switch(1, 0) → right-lte=0 → stdout "0", add/i1
                          left-gt=1 → compare_switch/left, compare/left
  add(0, 1) → 1 → output-tap/data. Consumed i2=1 → step-tap/data.
  output-tap(1, true) → passes → compare_switch/right, compare/right
  step-tap(1, true) → passes → add/i2

Cycle 2 (equal, limit=1 == current=1):
  compare(1, 1) → gt=false → step-tap/control, output-tap/control
  compare_switch(1, 1) → right-lte=1 → stdout "1", add/i1
                          right-gte=1 → output/last
                          left-gt does NOT fire (1 is not > 1)
  add(1, 1) → 2 → output-tap/data. Consumed i2=1 → step-tap/data.
  output-tap(2, false) → DISCARDS. compare_switch/right CLEAN. ✓
  step-tap(1, false) → DISCARDS. add/i2 CLEAN. ✓

Flow idle. All inputs clean. No stale values.
```

Output: 0, 1 ✓

When the flow restarts for the next outer iteration (limit=2, step=2, start=0):
- All inputs are clean from the previous iteration
- Always-initializer re-sends start=0
- External senders deliver new limit=2 and step=2
- Sequence runs correctly: 0, 2

Full sequence-of-sequences output: 0,1, 0,2, 0,3, ..., 0,9 ✓
