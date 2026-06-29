------------------------ MODULE AlwaysInit ------------------------
(*
 * Scenario: Function Always re-initialization after execution.
 *
 * Flow 10 contains p1 and p2.
 *   p1 has 2 inputs:
 *     input 0: function Always(1) — re-fires after every RetireAndSend
 *     input 1: function Once(1) — consumed once, never refilled
 *   p1 connects internally to p2 input 0.
 *   p2 has 1 input (no initializer), no outgoing connections.
 *
 * At startup both inputs are initialized (Always and Once both fire
 * for first_time=true).  p1 runs once.  RetireAndSend re-applies
 * Always on input 0 (external append).  But input 1 is empty (Once
 * consumed), so p1 cannot run again.  p2 runs from p1's output.
 *
 * AlwaysRefires verifies that whenever p1 is idle and not completed,
 * input 0 always has a value from the Always re-initialization.
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

NoParent == -1
NoInit == -2

VARIABLES inputQ, intCount, busyCount, ready, running, done, jobCounter

FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {1, 2},
    Flows <- {10},
    InputsOf <- 1 :> {0, 1} @@ 2 :> {0},
    Conns <- {[src |-> 1, dst |-> 2, dstInput |-> 0, internal |-> TRUE]},
    Parent <- 1 :> 10 @@ 2 :> 10 @@ 10 :> NoParent,
    InitOnce <- 1 :> (0 :> NoInit @@ 1 :> 1) @@ 2 :> (0 :> NoInit),
    InitAlways <- 1 :> (0 :> 1 @@ 1 :> NoInit) @@ 2 :> (0 :> NoInit),
    FlowInitOnce <- 1 :> (0 :> NoInit @@ 1 :> NoInit) @@ 2 :> (0 :> NoInit),
    FlowInitAlways <- 1 :> (0 :> NoInit @@ 1 :> NoInit) @@ 2 :> (0 :> NoInit),
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

AlwaysRefires ==
    (1 \notin done /\ ~FR!IsBusy(1))
    => Len(inputQ[1][0]) > 0

========================================================================
