--------------------- MODULE FlowAlwaysInit ---------------------
(*
 * Scenario: Flow-level Always re-initialization via FlowGoesIdle.
 *
 * Flow 10 contains p1 with 2 inputs:
 *   input 0: flow-level Always(1) — re-applied when flow goes idle
 *   input 1: function-level Once(1) — consumed once, bounds execution
 *
 * p1 sends internally to itself on input 0 (self-loop), creating
 * an internal value that triggers FlowGoesIdle when the flow
 * becomes idle.
 *
 * Lifecycle:
 * 1. Startup: p1 gets FlowInitAlways(1) on input 0, Once(1) on input 1
 * 2. p1 runs, self-loop sends internal value to input 0
 * 3. Flow 10 idle.  HasRunnableOnInternal = false (input 1 empty)
 * 4. FlowGoesIdle: clears internal on input 0, re-applies FlowInitAlways
 * 5. p1 has input 0 = <<1>> but input 1 empty — cannot run.  Terminates.
 *
 * FlowAlwaysRefires verifies that after FlowGoesIdle fires (no internal
 * values remain), input 0 always has a value from FlowInitAlways.
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

NoParent == -1
NoInit == -2

VARIABLES inputQ, intCount, busyCount, ready, running, done, jobCounter

FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {1},
    Flows <- {10},
    InputsOf <- 1 :> {0, 1},
    Conns <- {[src |-> 1, dst |-> 1, dstInput |-> 0, internal |-> TRUE]},
    Parent <- 1 :> 10 @@ 10 :> NoParent,
    InitOnce <- 1 :> (0 :> NoInit @@ 1 :> 1),
    InitAlways <- 1 :> (0 :> NoInit @@ 1 :> NoInit),
    FlowInitOnce <- 1 :> (0 :> NoInit @@ 1 :> NoInit),
    FlowInitAlways <- 1 :> (0 :> 1 @@ 1 :> NoInit),
    NoParent <- NoParent,
    NoInit <- NoInit,
    inputQ <- inputQ,
    intCount <- intCount,
    busyCount <- busyCount,
    ready <- ready,
    running <- running,
    done <- done,
    jobCounter <- jobCounter

Init == FR!Init
Next == FR!Next
Spec == FR!Spec

TypeOK == FR!TypeOK
CompletedNeverRuns == FR!CompletedNeverRuns
InternalCountBound == FR!InternalCountBound
AncestorConsistency == FR!AncestorConsistency

FlowAlwaysRefires ==
    (1 \notin done /\ ~FR!IsBusy(1) /\ intCount[1][0] = 0)
    => Len(inputQ[1][0]) > 0

========================================================================
