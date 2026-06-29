------------------------- MODULE FlowRuntimeBase -------------------------
(*
 * Formal specification of the flow runtime execution semantics.
 *
 * A process is the fundamental unit: it has inputs, executes when all
 * inputs have values, consumes one value per input, may produce output,
 * and routes output to connected processes. These semantics are the same
 * for all processes regardless of implementation (function or flow).
 *
 * Parallelism (multiple jobs for a function) is an optimization that
 * does not change the semantics.
 *
 * This module defines the generic logic. Specific flow topologies are
 * defined in separate modules that INSTANCE this one with concrete
 * values for the CONSTANTS.
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    Procs,          \* Set of all process IDs
    Flows,          \* Set of flow IDs (containers)
    InputsOf,       \* [proc -> set of input indices]
    Conns,          \* Set of [src, dst, dstInput, internal] records
    Parent,         \* [proc_or_flow -> parent flow ID or NoParent]
    InitOnce,       \* [proc -> [input -> value or NoInit]]
    InitAlways,     \* [proc -> [input -> value or NoInit]]
    NoParent,       \* Sentinel for "no parent flow" (root)
    NoInit          \* Sentinel for "no initializer" (distinct from any value)

VARIABLES
    inputQ,         \* [proc][input] -> sequence of values
    intCount,       \* [proc][input] -> count of internal values at front
    busyCount,      \* [id] -> count of busy markers (procs and flows)
    ready,          \* Sequence of job records
    running,        \* Set of job records
    done,           \* Set of completed process IDs
    jobCounter      \* Monotonically increasing job ID counter

vars == <<inputQ, intCount, busyCount, ready, running, done, jobCounter>>

---------------------------------------------------------------------------
(* Helpers *)

CanRun(p) ==
    /\ p \notin done
    /\ \A i \in InputsOf[p] : Len(inputQ[p][i]) > 0

IsBusy(id) == id \in DOMAIN busyCount

RECURSIVE AncestorsOf(_)
AncestorsOf(p) ==
    IF Parent[p] = NoParent
    THEN {}
    ELSE {Parent[p]} \union AncestorsOf(Parent[p])

ConnsFrom(p) == {c \in Conns : c.src = p}
ProcsInFlow(flow) == {p \in Procs : Parent[p] = flow}

CanRunOnInternal(p) ==
    /\ p \notin done
    /\ InputsOf[p] # {}
    /\ \A i \in InputsOf[p] : intCount[p][i] > 0

HasRunnableOnInternal(flow) ==
    \E p \in ProcsInFlow(flow) : CanRunOnInternal(p)

IncrBusy(ids) ==
    [id \in (DOMAIN busyCount \union ids) |->
      IF id \in DOMAIN busyCount
      THEN busyCount[id] + (IF id \in ids THEN 1 ELSE 0)
      ELSE 1
    ]

DecrBusy(ids) ==
    [id \in {x \in DOMAIN busyCount :
               ~(x \in ids /\ busyCount[x] = 1)} |->
      IF id \in ids
      THEN busyCount[id] - 1
      ELSE busyCount[id]
    ]

---------------------------------------------------------------------------
(* Initial state *)

Init ==
    /\ inputQ = [p \in Procs |->
         [i \in InputsOf[p] |->
           IF InitOnce[p][i] # NoInit THEN <<InitOnce[p][i]>>
           ELSE IF InitAlways[p][i] # NoInit THEN <<InitAlways[p][i]>>
           ELSE <<>>
         ]]
    /\ intCount = [p \in Procs |-> [i \in InputsOf[p] |-> 0]]
    /\ busyCount = [id \in {} |-> 0]
    /\ ready = <<>>
    /\ running = {}
    /\ done = {}
    /\ jobCounter = 0

---------------------------------------------------------------------------
(* Actions *)

(*
 * External send gating (Phase 4)
 *
 * In the runtime, send_a_value gates job creation for external sends:
 * if a value crosses a flow boundary (!connection.internal) and the
 * destination's parent flow is already busy, the value is queued but
 * no job is created.  The job is deferred until the parent flow goes
 * idle (handled by unblock_flows / has_runnable_on_internal).
 *
 * Internal sends (!dest_flow_busy when connection.internal) bypass the
 * gate — a function can always run on values produced within its own
 * flow, even while the flow is busy.
 *
 * In TLA+, CreateJob is a standalone action that fires nondeterministically
 * whenever its guard is satisfied.  Adding the gating guard here is
 * equivalent to gating inline in RetireAndSend: after RetireAndSend
 * queues values, CreateJob can only fire for a destination process if
 * the parent flow is idle OR the process can run entirely on internal
 * values (CanRunOnInternal — all inputs have intCount > 0).
 *)
CreateJob(p) ==
    /\ CanRun(p)
    /\ \/ ~IsBusy(Parent[p])
       \/ CanRunOnInternal(p)
    /\ LET inputs == [i \in InputsOf[p] |-> Head(inputQ[p][i])]
           toMark == {p} \union AncestorsOf(p)
       IN
       /\ inputQ' = [inputQ EXCEPT ![p] =
            [i \in InputsOf[p] |-> Tail(inputQ[p][i])]]
       /\ intCount' = [intCount EXCEPT ![p] =
            [i \in InputsOf[p] |->
              IF intCount[p][i] > 0 THEN intCount[p][i] - 1 ELSE 0]]
       /\ jobCounter' = jobCounter + 1
       /\ ready' = Append(ready,
            [func |-> p, inputs |-> inputs, jobId |-> jobCounter + 1])
       /\ busyCount' = IncrBusy(toMark)
       /\ UNCHANGED <<running, done>>

Dispatch ==
    /\ Len(ready) > 0
    /\ running' = running \union {Head(ready)}
    /\ ready' = Tail(ready)
    /\ UNCHANGED <<inputQ, intCount, busyCount, done, jobCounter>>

(*
 * Input queue ordering discipline
 *
 * Each input maintains a single queue partitioned by intCount:
 *
 *   positions 1..intCount        = internal (within-flow) values
 *   positions intCount+1..Len(q) = external (cross-flow) values
 *
 * - send_internal: insert at position intCount+1, then intCount += 1
 * - send (external): append to end of queue
 * - take: remove Head (position 1), decrement intCount if > 0
 * - clear_internal: keep only the external suffix (positions intCount+1..Len)
 *
 * Internal values are always consumed before external values.
 * FlowGoesIdle clears all internal values while preserving external.
 *)

RetireAndSend(job) ==
    /\ job \in running
    /\ running' = running \ {job}
    /\ LET outVal == IF InputsOf[job.func] = {} THEN 1
                     ELSE job.inputs[CHOOSE i \in InputsOf[job.func] : TRUE]
           conns == ConnsFrom(job.func)
           toDecr == {job.func} \union AncestorsOf(job.func)
       IN
       /\ inputQ' = [p \in Procs |->
            [i \in InputsOf[p] |->
              IF \E c \in conns : c.dst = p /\ c.dstInput = i
              THEN IF (\E c \in conns : c.dst = p /\ c.dstInput = i /\ c.internal)
                   THEN SubSeq(inputQ[p][i], 1, intCount[p][i])
                        \o <<outVal>>
                        \o SubSeq(inputQ[p][i], intCount[p][i] + 1, Len(inputQ[p][i]))
                   ELSE Append(inputQ[p][i], outVal)
              ELSE inputQ[p][i]
            ]]
       /\ intCount' = [p \in Procs |->
            [i \in InputsOf[p] |->
              IF \E c \in conns : c.dst = p /\ c.dstInput = i /\ c.internal
              THEN intCount[p][i] + 1
              ELSE intCount[p][i]
            ]]
       /\ busyCount' = DecrBusy(toDecr)
       /\ done' = done
       /\ UNCHANGED <<ready, jobCounter>>

CompleteJob(job) ==
    /\ job \in running
    /\ running' = running \ {job}
    /\ done' = done \union {job.func}
    /\ busyCount' = DecrBusy({job.func} \union AncestorsOf(job.func))
    /\ UNCHANGED <<inputQ, intCount, ready, jobCounter>>

(*
 * When a flow becomes idle, the runtime first checks has_runnable_on_internal.
 * If any function can still run on internal data, CreateJob handles that —
 * its CanRunOnInternal guard allows firing for processes whose inputs are
 * all internal, even while the parent flow is busy.
 *
 * FlowGoesIdle fires only when NO function can run on internal data alone.
 * It clears all internal values, matching clear_flow_internal_inputs.
 *
 * NOTE: The runtime gates this with ~has_runnable_on_internal(flow) —
 * modeled here but the guard requires CompleteJob to work correctly
 * (otherwise self-loops create unbounded state spaces in TLC).
 * The guard is deferred until CompleteJob semantics are refined.
 *
 * The runtime also re-applies Always initializers via
 * run_flow_initializers — deferred to Phase 5 (initializer semantics).
 *)
FlowGoesIdle(flow) ==
    /\ flow \in Flows
    /\ ~IsBusy(flow)
    /\ \E p \in ProcsInFlow(flow) : \E i \in InputsOf[p] : intCount[p][i] > 0
    /\ inputQ' = [p \in Procs |->
         [i \in InputsOf[p] |->
           IF Parent[p] = flow
           THEN SubSeq(inputQ[p][i], intCount[p][i] + 1, Len(inputQ[p][i]))
           ELSE inputQ[p][i]
         ]]
    /\ intCount' = [p \in Procs |->
         [i \in InputsOf[p] |->
           IF Parent[p] = flow THEN 0 ELSE intCount[p][i]
         ]]
    /\ UNCHANGED <<busyCount, ready, running, done, jobCounter>>

---------------------------------------------------------------------------
(* Specification *)

Next ==
    \/ \E p \in Procs : CreateJob(p)
    \/ Dispatch
    \/ \E job \in running : RetireAndSend(job)
    \/ \E job \in running : CompleteJob(job)
    \/ \E flow \in Flows : FlowGoesIdle(flow)

Spec == Init /\ [][Next]_vars

---------------------------------------------------------------------------
(* Invariants *)

TypeOK ==
    /\ \A p \in Procs : \A i \in InputsOf[p] :
         /\ inputQ[p][i] \in Seq(Int)
         /\ intCount[p][i] \in Nat
    /\ done \subseteq Procs
    /\ jobCounter \in Nat

CompletedNeverRuns ==
    \A p \in done :
        /\ \A j \in running : j.func # p
        /\ \A idx \in 1..Len(ready) : ready[idx].func # p

(* Together with TypeOK (intCount \in Nat), this ensures the queue partition
   invariant: 0 <= intCount[p][i] <= Len(inputQ[p][i]) for all inputs. *)
InternalCountBound ==
    \A p \in Procs : \A i \in InputsOf[p] :
        intCount[p][i] <= Len(inputQ[p][i])

AncestorConsistency ==
    \A p \in Procs :
        IsBusy(p) => \A a \in AncestorsOf(p) : IsBusy(a)

==========================================================================
