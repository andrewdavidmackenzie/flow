--------------------- MODULE InternalExternal ---------------------
(*
 * Scenario: A function with one internal and one external input.
 * Exercises intCount tracking when internal and external values
 * coexist in different inputs of the same function.
 *
 * Flow 10 contains p1 and p2.
 *   p1 has 1 input (Once-initialized), connects internally to p2 input 0.
 *   p2 has 2 inputs: input 0 from p1 (internal), input 1 from p3 (external).
 *   p3 is in root flow 0 with 1 input (Once-initialized), connects
 *   externally to p2 input 1.
 *
 * p2 needs both inputs to run. Input 0 has intCount tracked (internal),
 * input 1 does not (external). Verifies InternalCountBound and
 * AncestorConsistency hold across all interleavings.
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

NoParent == -1
NoInit == -2

VARIABLES inputQ, intCount, busyCount, ready, running, done, jobCounter

FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {1, 2, 3},
    Flows <- {0, 10},
    InputsOf <- 1 :> {0} @@ 2 :> {0, 1} @@ 3 :> {0},
    Conns <- { [src |-> 1, dst |-> 2, dstInput |-> 0, internal |-> TRUE],
               [src |-> 3, dst |-> 2, dstInput |-> 1, internal |-> FALSE] },
    Parent <- 1 :> 10 @@ 2 :> 10 @@ 3 :> 0 @@ 10 :> 0 @@ 0 :> NoParent,
    InitOnce <- 1 :> (0 :> 1) @@ 2 :> (0 :> NoInit @@ 1 :> NoInit) @@ 3 :> (0 :> 1),
    InitAlways <- 1 :> (0 :> NoInit) @@ 2 :> (0 :> NoInit @@ 1 :> NoInit) @@ 3 :> (0 :> NoInit),
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

======================================================================
