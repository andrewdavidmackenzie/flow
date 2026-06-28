------------------------ MODULE MixedQueue ------------------------
(*
 * Scenario: Feedback loop (internal self-connection) plus external input.
 * Exercises FlowGoesIdle clearing internal values while preserving external.
 *
 * Flow 10 contains p1 with:
 *   - input 0: Once-initialized, receives internal feedback from p1 itself
 *   - input 1: receives external values from p2
 *
 * p2 is in root flow 0 with Once-initialized input.
 *
 * When p1 runs on its Once value, it sends output back to itself (internal)
 * AND externally. When flow 10 goes idle, internal values should be cleared
 * but the external value from p2 should be preserved.
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

NoParent == -1
NoInit == -2

VARIABLES inputQ, intCount, busyCount, ready, running, done, jobCounter

FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {1, 2},
    Flows <- {0, 10},
    InputsOf <- 1 :> {0, 1} @@ 2 :> {0},
    Conns <- { [src |-> 1, dst |-> 1, dstInput |-> 0, internal |-> TRUE],
               [src |-> 2, dst |-> 1, dstInput |-> 1, internal |-> FALSE] },
    Parent <- 1 :> 10 @@ 2 :> 0 @@ 10 :> 0 @@ 0 :> NoParent,
    InitOnce <- 1 :> (0 :> 1 @@ 1 :> NoInit) @@ 2 :> (0 :> 1),
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

========================================================================
