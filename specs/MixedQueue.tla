------------------------ MODULE MixedQueue ------------------------
(*
 * Scenario: Internal self-feedback and external send to the SAME input,
 * with a second input that bounds execution.
 *
 * Flow 10 contains p1 with 2 inputs:
 *   input 0: Once-initialized, receives internal feedback from p1 itself
 *             AND external values from p2 — exercises mixed queue.
 *   input 1: Once-initialized, no connections — consumed once, never refilled,
 *             so p1 runs at most once (bounding the state space).
 *
 * p2 is in root flow 0 with 1 input (Once-initialized),
 * connects externally to p1 input 0.
 *
 * After p1 runs, the self-loop queues an internal value on input 0.
 * If p2 also sent an external value, input 0 has both internal and
 * external values. When FlowGoesIdle(10) fires, internal values are
 * cleared but the external value is preserved.
 * p1 cannot run again because input 1 is empty.
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
               [src |-> 2, dst |-> 1, dstInput |-> 0, internal |-> FALSE] },
    Parent <- 1 :> 10 @@ 2 :> 0 @@ 10 :> 0 @@ 0 :> NoParent,
    InitOnce <- 1 :> (0 :> 1 @@ 1 :> 1) @@ 2 :> (0 :> 1),
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
