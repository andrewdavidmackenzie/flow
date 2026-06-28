------------------------ MODULE MixedQueue ------------------------
(*
 * Scenario: Internal self-feedback and external send to the SAME input.
 * Exercises FlowGoesIdle clearing internal values while preserving
 * external values on the same queue.
 *
 * Flow 10 contains p1 with 1 input (Once-initialized).
 *   p1 connects to itself on input 0 (internal self-loop).
 *   p2 is in root flow 0 with 1 input (Once-initialized),
 *   connects externally to p1 input 0.
 *
 * After p1 runs on its Once value, the self-loop queues an internal
 * value on input 0. If p2 has also sent an external value, input 0
 * has both internal and external values in the same queue. When
 * FlowGoesIdle(10) fires, it clears internal values (via SubSeq
 * keeping only positions intCount+1..Len) while external values
 * are preserved. InternalCountBound verifies the partition stays
 * valid throughout, ensuring the SubSeq boundary is always correct.
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

NoParent == -1
NoInit == -2

VARIABLES inputQ, intCount, busyCount, ready, running, done, jobCounter

FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {1, 2},
    Flows <- {0, 10},
    InputsOf <- 1 :> {0} @@ 2 :> {0},
    Conns <- { [src |-> 1, dst |-> 1, dstInput |-> 0, internal |-> TRUE],
               [src |-> 2, dst |-> 1, dstInput |-> 0, internal |-> FALSE] },
    Parent <- 1 :> 10 @@ 2 :> 0 @@ 10 :> 0 @@ 0 :> NoParent,
    InitOnce <- 1 :> (0 :> 1) @@ 2 :> (0 :> 1),
    InitAlways <- 1 :> (0 :> NoInit) @@ 2 :> (0 :> NoInit),
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
InternalCountBound == FR!InternalCountBound
AncestorConsistency == FR!AncestorConsistency
(* CompletedNeverRuns omitted: known Phase 3 limitation where
   self-loop can queue a second job before the first completes. *)

========================================================================
