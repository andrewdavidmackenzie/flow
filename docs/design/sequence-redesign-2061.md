# Sequence Flow Redesign for Nested Loops (#2061)

## Problem

When the sequence sub-flow is used nested (sequence-of-sequences), stale loopback values on
add:i2 (step feedback) persist across flow idle boundaries, corrupting the next iteration.

## Solution: compare + tap gated step feedback

Replace the unconditional `add/i2 → add/i2` loopback with a gated feedback path using two
new functions in the sequence flow:

1. **compare** (`lib://flowstdlib/math/compare`) — fed the same left (limit) and right (current)
   inputs as compare_switch. Outputs boolean `gt` which is `true` when iterating (left > right)
   and `false` on the equal or exceeded case.

2. **tap** (`lib://flowstdlib/control/tap`) — gates the step feedback. Takes add's i2 as data
   and compare's `gt` as control. When `gt=true`, passes step through to add:i2. When `gt=false`,
   discards the step — preventing it from sitting stale on add:i2.

### Why right-lte (not right-lt) for add:i1

The tap needs a chance to fire with `gt=false` to DISCARD the last step value. For this to happen,
add must run one more time after the equal case, sending its i2 to tap:data. This requires add:i1
to receive a value on the equal case.

- With `right-lt` → add:i1: add doesn't get a value on equal → doesn't run → tap has no data →
  can't discard → stale step remains on add:i2.
- With `right-lte` → add:i1: add DOES get a value on equal → runs → sends i2 to tap → tap fires
  with [step, false] → discards → add:i2 is clean.

The extra add run produces a value that exceeds the limit. This feeds back to compare_switch which
sees right > left — no right-lte output to stdout, so no wrong output. The right-gte output goes
to output/last which is the correct "sequence done" signal.

### Limitation: non-equal termination

This approach only cleans add:i2 when the sequence terminates via the equal case (limit hit
exactly). When the sequence terminates via exceeded (step doesn't divide evenly into limit-start),
there's no equal case, add doesn't get the extra run, and the stale step remains.

This is acceptable because:
- The simple sequence (non-nested) doesn't restart, so stale values don't matter.
- In sequence-of-sequences, the outer sends the same value as both step and limit, so the inner
  always terminates on equal.
- A future improvement could handle the exceeded case by also connecting right-gte to add:i1
  (or by using a different gating mechanism).

## Sequence flow structure (with compare + tap)

```
Functions: compare_switch, compare, add, step-tap
Connections:
  input/start   → compare_switch/right, compare/right  (first value)
  input/limit   → compare_switch/left, compare/left    (limit)
  input/step    → add/i2                                (initial step)

  compare_switch/right-lte → output/number              (stdout)
  compare_switch/right-lte → add/i1                     (previous value for add)
  compare_switch/left-gt   → compare_switch/left        (limit loopback)
  compare_switch/left-gt   → compare/left               (limit loopback to compare too)
  compare_switch/right-gte → output/last                 (sequence done signal)

  add output               → compare_switch/right       (next value)
  add output               → compare/right              (next value to compare too)

  add/i2                   → step-tap/data               (step to tap)
  compare/gt               → step-tap/control            (boolean gate)
  step-tap output          → add/i2                      (gated step feedback)
```

## Trace analysis

For limit=1, step=1 inner iteration:

```
Cycle 1 (keep going, limit=1 > current=0):
  compare [1,0] → gt=true → tap:control
  compare_switch [1,0] → right-lte=0 → add:i1, stdout "0"
  add [0, 1] → 1. Sends i2=1 → tap:data.
  tap [1, true] → passes → add:i2=1.

Cycle 2 (equal, limit=1 == current=1):
  compare [1,1] → gt=false → tap:control
  compare_switch [1,1] → right-lte=1 → add:i1=1, stdout "1"
  add [1, 1] → 2. Sends i2=1 → tap:data.
  tap [1, false] → DISCARDS. add:i2 EMPTY. ✓

Cycle 3 (exceeded, limit=1 < current=2):
  compare [1,2] → gt=false. compare_switch [1,2] → no right-lte.
  add:i1 empty. Add can't run. Flow idle.
  add:i2 EMPTY. ✓  No stale values.
```

Output: 0, 1 ✓

## Current status: tap fires with stale control

The expected trace above assumes compare and compare_switch process in lockstep. In practice,
compare and compare_switch are independent functions that receive the same inputs but process
at different times. This causes a FIFO ordering issue in tap's inputs.

### What actually happens (from trace with -j 1)

```
Job #4:  compare [1, 0] → gt=true → tap:control           (cycle 1 control)
Job #7:  add [0, 1] → 1. Sends i2=1 → tap:data            (cycle 1 data)
Job #11: step-tap [1, Bool(true)] → Some(1) → add:i2      (fires with cycle 1 control+data) ✓

Job #10: compare [1, 1] → gt=false → tap:control           (cycle 2 control)
Job #12: compare_switch [1,1] → equal → right-lte=1 → add:i1
Job #14: add [1, 1] → 2. Sends i2=1 → tap:data            (cycle 2 data)
Job #15: step-tap [1, Bool(false)] → None                   (fires with cycle 2 control, DISCARDS) ✓

Flow #2 idle. add:i2 EMPTY. ✓
```

The tap fires correctly: passes on cycle 1 (gt=true), discards on cycle 2 (gt=false). The final
state is clean — add:i2 is empty after flow idle.

BUT: between Job #11 and Job #15, add:i2 received step=1 from tap (Job #11). This value was
consumed by add in Job #14 (add [1,1] → 2). That's correct — the step was used.

The actual output from the nested sequence is wrong (17 lines, values off) even though the
mechanism works for the first inner iteration. The problem may be in:
1. The compare receiving wrong input values on subsequent iterations (wiring issue)
2. The tap FIFO accumulating controls from multiple cycles across iterations
3. Stale values on compare's own inputs (left loopback from compare_switch)

### Next steps

1. Trace the SECOND inner iteration (limit=2) to find where values diverge
2. Check that compare/left receives the limit loopback correctly from compare_switch/left-gt
3. Check that tap:control doesn't accumulate stale booleans across flow idle boundaries
4. Verify that compare and compare_switch both receive the same right input at the same time
