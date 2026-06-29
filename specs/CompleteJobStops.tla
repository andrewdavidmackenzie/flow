--------------------- MODULE CompleteJobStops ---------------------
(*
 * Scenario: CompleteJob prevents re-execution.
 *
 * Flow 10 contains p1 with 1 input (function Always(1)).
 * p1 has no outgoing connections.
 *
 * Without CompleteJob, p1 would loop forever: Always re-fires after
 * each RetireAndSend, making p1 runnable again.  CompleteJob provides
 * the termination path — once p1 is in `done`, CanRun fails and no
 * more jobs are created despite the Always initializer.
 *
 * CompletedStaysIdle verifies that once p1 completes, it never has
 * another job in ready or running.
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

NoParent == -1
NoInit == -2

VARIABLES inputQ, intCount, busyCount, ready, running, done, jobCounter

FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {1},
    Flows <- {10},
    InputsOf <- 1 :> {0},
    Conns <- {},
    Parent <- 1 :> 10 @@ 10 :> NoParent,
    InitOnce <- 1 :> (0 :> NoInit),
    InitAlways <- 1 :> (0 :> 1),
    FlowInitOnce <- 1 :> (0 :> NoInit),
    FlowInitAlways <- 1 :> (0 :> NoInit),
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

StateConstraint == jobCounter <= 8

CompletedStaysIdle ==
    1 \in done =>
        /\ \A j \in running : j.func # 1
        /\ \A idx \in 1..Len(ready) : ready[idx].func # 1

========================================================================
