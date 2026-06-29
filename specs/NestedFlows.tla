----------------------- MODULE NestedFlows -----------------------
(*
 * Scenario: Nested flow hierarchy — flow 10 inside flow 0.
 *
 * Flow 0 (root) contains p3 and flow 10.
 * Flow 10 contains p1 and p2.
 *   p1 has 1 input (Once-initialized), connects internally to p2.
 *   p2 has 1 input (no init), connects externally to p3.
 *   p3 has 1 input (no init), no outgoing connections.
 *
 * Exercises ancestor busy-count propagation across two nesting levels:
 * when p1 runs, both flow 10 and flow 0 must be marked busy.
 * When p2 sends externally to p3, the external gating check applies
 * (Parent[p3] = 0, flow 0 may or may not be busy).
 *
 * AncestorConsistency verifies that busy functions always have busy
 * ancestors at every nesting level.
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

NoParent == -1
NoInit == -2

VARIABLES inputQ, intCount, busyCount, ready, running, done, jobCounter

FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {1, 2, 3},
    Flows <- {0, 10},
    InputsOf <- 1 :> {0} @@ 2 :> {0} @@ 3 :> {0},
    Conns <- { [src |-> 1, dst |-> 2, dstInput |-> 0, internal |-> TRUE],
               [src |-> 2, dst |-> 3, dstInput |-> 0, internal |-> FALSE] },
    Parent <- 1 :> 10 @@ 2 :> 10 @@ 3 :> 0 @@ 10 :> 0 @@ 0 :> NoParent,
    InitOnce <- 1 :> (0 :> 1) @@ 2 :> (0 :> NoInit) @@ 3 :> (0 :> NoInit),
    InitAlways <- 1 :> (0 :> NoInit) @@ 2 :> (0 :> NoInit) @@ 3 :> (0 :> NoInit),
    FlowInitOnce <- 1 :> (0 :> NoInit) @@ 2 :> (0 :> NoInit) @@ 3 :> (0 :> NoInit),
    FlowInitAlways <- 1 :> (0 :> NoInit) @@ 2 :> (0 :> NoInit) @@ 3 :> (0 :> NoInit),
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

=====================================================================
