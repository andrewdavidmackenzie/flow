--------------------- MODULE InternalExternal ---------------------
(*
 * Scenario: Mixed internal and external sends to the SAME input.
 * Exercises intCount tracking when both internal and external values
 * coexist in a single input queue.
 *
 * Flow 10 contains p1 and p2.
 *   p1 has 1 input (Once-initialized), connects internally to p2 input 0.
 *   p2 has 1 input (no initializer), receives from p1 (internal) AND
 *   p3 (external) on the same input 0.
 *   p3 is in root flow 0 with 1 input (Once-initialized), connects
 *   externally to p2 input 0.
 *
 * p2 input 0 will have both internal (intCount-tracked) and external
 * values in the same queue, verifying InternalCountBound holds when
 * the insert-at-intCount+1 ordering is exercised.
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
               [src |-> 3, dst |-> 2, dstInput |-> 0, internal |-> FALSE] },
    Parent <- 1 :> 10 @@ 2 :> 10 @@ 3 :> 0 @@ 10 :> 0 @@ 0 :> NoParent,
    InitOnce <- 1 :> (0 :> 1) @@ 2 :> (0 :> NoInit) @@ 3 :> (0 :> 1),
    InitAlways <- 1 :> (0 :> NoInit) @@ 2 :> (0 :> NoInit) @@ 3 :> (0 :> NoInit),
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
   multiple values on one input can create multiple jobs and
   CompleteJob may fire on the first while the second is queued. *)

======================================================================
