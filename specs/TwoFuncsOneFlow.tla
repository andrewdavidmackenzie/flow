------------------------ MODULE TwoFuncsOneFlow ------------------------
(*
 * Scenario: Two processes in one flow.
 * p1 has 2 inputs (both Once-initialized), connects to p2 input 0.
 * p2 has 1 input.
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
    InitOnce <- 1 :> (0 :> 1 @@ 1 :> 2) @@ 2 :> (0 :> NoInit),
    InitAlways <- 1 :> (0 :> NoInit @@ 1 :> NoInit) @@ 2 :> (0 :> NoInit),
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

==========================================================================
