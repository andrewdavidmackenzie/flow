----------------------- MODULE ExternalGating -----------------------
(*
 * Scenario: External send gating — value crosses a flow boundary
 * while the destination flow is busy.
 *
 * Flow 10 contains p1 and p2.
 *   p1 has 1 input (Once-initialized), connects internally to p2 input 0.
 *   p2 has 1 input (no initializer), no outgoing connections.
 *
 * p3 is in root flow 0 with 1 input (Once-initialized), connects
 * externally to p1 input 0.
 *
 * At startup p1 and p3 both run.  When p1 retires it sends internally
 * to p2, making p2 runnable.  When p3 retires it sends externally to
 * p1.  If flow 10 is still busy (p2 running), the external value for
 * p1 must be queued without creating a job — CreateJob's gating guard
 * blocks because Parent[p1] = 10 is busy and intCount = 0.
 *
 * Once flow 10 goes idle, CreateJob(p1) fires on the queued external
 * value.  This exercises the Phase 4 external send gating.
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
               [src |-> 3, dst |-> 1, dstInput |-> 0, internal |-> FALSE] },
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
(* CompletedNeverRuns omitted: p1 runs twice (init + external from p3),
   sending to p2 each time.  Two jobs for p2 can be queued, and
   CompleteJob may fire on the first while the second is still ready. *)

=====================================================================
