## Distributed Execution of Sub-flows

### Overview

`flowrcli` can distribute sub-flow execution across multiple machines. When a flow
contains a sub-flow whose functions are all library functions (`lib://`), that
sub-flow can be delegated to another `flowrcli` instance running on the same
machine or on a remote machine.

The delegating `flowrcli` extracts the sub-flow from the manifest, creates a
proxy function in its place, and sends the sub-flow manifest to the peer for
execution. Boundary outputs from the sub-flow stream back to the parent
coordinator as they are produced, and are routed to the destination functions
in the parent flow.

### How It Works

1. Start a second `flowrcli` instance with no flow manifest argument. This starts
   in coordinator-only mode, advertising itself as a peer coordinator via mDNS
   and waiting for sub-flow submissions.

2. Run your flow with `flowrcli --delegate`. The `--delegate` flag tells
   `flowrcli` to look for a peer coordinator on the network. If one is found,
   the largest eligible sub-flow is extracted and sent to the peer for execution.

3. If no peer is found, the flow runs normally without delegation.

### Which Sub-flows Are Eligible

A sub-flow is eligible for delegation when **every function** in its subtree
(including nested sub-flows) uses a `lib://` implementation. This means:

- Standard library functions (`lib://flowstdlib/*`) — eligible
- WASM functions (`file://`) — not eligible (yet)
- Context functions (`context://`) — not eligible (they interact with the local environment)

The sub-flow with the largest number of eligible functions is selected.

### Example: Two Terminals on the Same Machine

#### Terminal 1 — start a peer coordinator

```
flowrcli -v info
```

This starts `flowrcli` in coordinator-only mode (no flow manifest provided).
It advertises itself via mDNS and waits for sub-flow submissions.

#### Terminal 2 — compile and run a flow with delegation

```
flowc -c -O flowr/examples/mandlebrot
flowrcli --delegate -v info flowr/examples/mandlebrot/manifest.json -- output.png '[200,150]' '[[-1.20,0.35],[-1,0.20]]'
```

The `--delegate` flag causes `flowrcli` to:
1. Discover the peer coordinator started in Terminal 1
2. Extract the `generate_pixels` sub-flow (6 functions, all `lib://`)
3. Send the sub-flow to the peer for execution
4. Receive boundary outputs (pixel coordinates) as they are produced
5. Continue local execution (pixel-to-point, escapes, image rendering)

The resulting image is identical to running without `--delegate`.

### Cross-Machine Execution

The same approach works across machines. Start `flowrcli` (with no manifest) on a
remote machine on the same network. The mDNS discovery protocol will find it
automatically — no configuration needed.

```
# Machine A (peer coordinator)
flowrcli -v info

# Machine B (run the flow)
flowrcli --delegate -v info flowr/examples/mandlebrot/manifest.json -- output.png '[200,150]' '[[-1.20,0.35],[-1,0.20]]'
```

### No Peer Available

If `--delegate` is specified but no peer coordinator is found on the network,
the flow runs normally without any delegation overhead. A log message indicates
that no peers were discovered.

### Dynamic Peer Addition

Peers can be started at any time. The `--delegate` flag discovers peers at
startup. Future versions may support discovering peers mid-execution.

### Architecture

Each `flowrcli` peer instance runs its own coordinator with:
- A ZMQ REP socket for receiving sub-flow submissions
- Its own dispatcher and executor pool for running received sub-flows
- mDNS advertisement so parent coordinators can discover it

The parent coordinator communicates with the peer using the peer protocol:
- `PeerRequest::Submit` sends the sub-flow manifest and input values
- `PeerResponse::BoundaryOutput` streams each boundary output back
- `PeerResponse::Idle` signals sub-flow completion
