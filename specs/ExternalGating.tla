----------------------- MODULE ExternalGating -----------------------
(*
 * Scenario: External send gating — value crosses a flow boundary
 * while the destination flow is busy.
 *
 * Flow 10 contains p1 and p2.
 *   p1 has 1 input (no initializer), no outgoing connections.
 *   p2 has 1 input (Once-initialized), no outgoing connections.
 *
 * p3 is in root flow 0 with 1 input (Once-initialized), connects
 * externally to p1 input 0.
 *
 * At startup p2 and p3 both run (they have Once-initialized inputs).
 * p2 makes flow 10 busy.  When p3 retires it sends externally to p1.
 * While flow 10 is busy (p2 running), the external value for p1 must
 * be queued without creating a job — CreateJob's gating guard blocks
 * because Parent[p1] = 10 is busy and CanRunOnInternal(p1) is false.
 *
 * Once p2 retires and flow 10 goes idle, CreateJob(p1) fires on the
 * queued external value.
 *
 * BusyFlowBlocksExternalJob verifies this directly: whenever flow 10
 * is busy and p1 has only external values, p1 must have no jobs.
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

NoParent == -1
NoInit == -2

VARIABLES inputQ, intCount, busyCount, ready, running, done, jobCounter

FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {1, 2, 3},
    Flows <- {0, 10},
    InputsOf <- 1 :> {0} @@ 2 :> {0} @@ 3 :> {0},
    Conns <- { [src |-> 3, dst |-> 1, dstInput |-> 0, internal |-> FALSE] },
    Parent <- 1 :> 10 @@ 2 :> 10 @@ 3 :> 0 @@ 10 :> 0 @@ 0 :> NoParent,
    InitOnce <- 1 :> (0 :> NoInit) @@ 2 :> (0 :> 1) @@ 3 :> (0 :> 1),
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

BusyFlowBlocksExternalJob ==
    (FR!IsBusy(10) /\ Len(inputQ[1][0]) > 0 /\ intCount[1][0] = 0)
    => (/\ \A j \in running : j.func # 1
        /\ \A idx \in 1..Len(ready) : ready[idx].func # 1)

=====================================================================
