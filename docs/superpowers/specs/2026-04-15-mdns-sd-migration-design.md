# Replace simpdiscover with mdns-sd for Service Discovery

## Problem

The flow project uses `simpdiscover` (v0.7) for service discovery between the coordinator,
executors, and clients. This is a custom UDP beacon protocol that doesn't follow the mDNS
standard. The `mdns-sd` crate is a standard mDNS-SD implementation already used successfully
in the pigg project, including on macOS Sequoia CI runners (macos-26).

Switching to `mdns-sd` may also help resolve the macOS Sequoia Local Network Privacy issue
(#2560) since `mdns-sd` uses standard mDNS which may be handled differently by the OS than
custom UDP beacons.

## Current State

### Dependency
- `simpdiscover = "0.7"` in `flowr/Cargo.toml`

### Functions (duplicated across 3 binaries)

**`discover_service(discovery_port, name) -> Result<String>`**
- Creates a `BeaconListener`, calls `wait(None)` to block until found
- Returns `"{ip}:{port}"` string
- Duplicated in: `flowrcli/cli/connections.rs`, `flowrgui/gui/client_connection.rs`, `flowrex/main.rs`

**`enable_service_discovery(discovery_port, name, service_port) -> Result<()>`**
- Creates a `BeaconSender`, spawns a thread calling `send_loop(1s)`
- Duplicated in: `flowrcli/cli/connections.rs`, `flowrgui/gui/coordinator_connection.rs`

### Constants (in `flowr/src/lib/services.rs`)
- `JOB_SERVICE_NAME = "jobs._flowr._tcp.local"`
- `RESULTS_JOB_SERVICE_NAME = "results._flowr._tcp.local"`
- `CONTROL_SERVICE_NAME = "control._flowr._tcp.local"`
- `JOB_QUEUES_DISCOVERY_PORT = 15003`

Additional constants in connection modules:
- `COORDINATOR_SERVICE_NAME = "runtime._flowr._tcp.local"` (in both flowrcli and flowrgui)
- `DEBUG_SERVICE_NAME = "debug._flowr._tcp.local"` (in both flowrcli and flowrgui)

### Usage Sites
- `flowrcli/cli/connections.rs` — advertises coordinator/debug services, tests discover them
- `flowrcli/main.rs` — advertises job/results/control services (lines 293-295)
- `flowrgui/gui/coordinator_connection.rs` — advertises coordinator/debug services
- `flowrgui/gui/client_connection.rs` — discovers coordinator service
- `flowrgui/connection_manager.rs` — advertises job/results/control services (lines 175-177)
- `flowrex/main.rs` — discovers job/results/control services (lines 109-118)
- `flowrcli/cli/connections.rs` test code — uses both discover and advertise

## Design

### Principle: discovery is optional

`flowrlib` is designed to be usable by binary crates outside this workspace. The discovery
module and service name constants are conveniences — not obligations. Other binaries that
use `flowrlib` can implement their own discovery mechanism. Nothing in `flowrlib`'s core
(coordinator, executor, dispatcher) should depend on or require `mdns-sd`.

### New file: `flowr/src/lib/discovery.rs`

Create a single module in `flowrlib` with two public functions that replace all duplicated
discovery code:

```rust
/// Discover a service by name. Blocks until the service is found.
/// Returns the service address as "{ip}:{port}".
pub fn discover_service(name: &str) -> Result<String>
```

```rust
/// Register a service for discovery. The ServiceDaemon runs in a background thread
/// and continuously responds to mDNS queries. Returns the daemon handle so the
/// caller can keep it alive (service is unregistered when dropped).
pub fn enable_service_discovery(name: &str, service_port: u16) -> Result<ServiceDaemon>
```

Key changes from current API:
- No more `discovery_port` parameter — mdns-sd uses standard mDNS port 5353 automatically
- `enable_service_discovery` returns `ServiceDaemon` instead of `()` — the caller must keep
  the handle alive to keep the service registered. When dropped, the service is unregistered.
- Uses synchronous `recv()` on the browse channel, not async

### Changes to `flowr/src/lib/services.rs`

- Remove `JOB_QUEUES_DISCOVERY_PORT` constant (no longer needed)
- Move `COORDINATOR_SERVICE_NAME` and `DEBUG_SERVICE_NAME` here from the connection modules
  (they're currently duplicated between flowrcli and flowrgui)
- Service name format: keep existing `"name._flowr._tcp.local."` format, adding trailing dot
  per mDNS convention if needed

### Changes to `flowr/src/lib/lib.rs`

- Add `pub mod discovery;`

### Changes to `flowr/Cargo.toml`

- Remove `simpdiscover = "0.7"`
- Add `mdns-sd = "0.19"` (latest, no async runtime dependency)

### Changes to binary crates

Each binary replaces its local `discover_service`/`enable_service_discovery` functions with
imports from `flowrlib::discovery`:

**`flowrcli/cli/connections.rs`**
- Remove `use simpdiscoverylib::{BeaconListener, BeaconSender}`
- Remove local `discover_service()` and `enable_service_discovery()` functions
- Remove `COORDINATOR_SERVICE_NAME` and `DEBUG_SERVICE_NAME` constants
- Import from `flowrlib::discovery::{discover_service, enable_service_discovery}`
- Import service name constants from `flowrlib::services`
- Update callers: drop `discovery_port` argument
- Store returned `ServiceDaemon` handles to keep services registered

**`flowrcli/main.rs`**
- Remove `JOB_QUEUES_DISCOVERY_PORT` from services import
- Update 3 calls to `enable_service_discovery`: drop `discovery_port`, store returned handles

**`flowrgui/gui/client_connection.rs`**
- Remove `use simpdiscoverylib::BeaconListener`
- Remove local `discover_service()` function
- Import from `flowrlib::discovery::discover_service`

**`flowrgui/gui/coordinator_connection.rs`**
- Remove `use simpdiscoverylib::BeaconSender`
- Remove local `enable_service_discovery()` function
- Remove `COORDINATOR_SERVICE_NAME` and `DEBUG_SERVICE_NAME` constants
- Import from `flowrlib::discovery::enable_service_discovery`
- Import service name constants from `flowrlib::services`
- Store returned `ServiceDaemon` handle

**`flowrgui/connection_manager.rs`**
- Update 3 calls to `enable_service_discovery`: drop `discovery_port`, store returned handles

**`flowrex/main.rs`**
- Remove `use simpdiscoverylib::BeaconListener`
- Remove local `discover_service()` function
- Remove `JOB_QUEUES_DISCOVERY_PORT` from services import
- Import from `flowrlib::discovery::discover_service`
- Update 3 calls to `discover_service`: drop `discovery_port`

### Test changes

**`flowrcli/cli/connections.rs` tests**
- Update `enable_service_discovery` and `discover_service` calls to use new API
- The test helper `wait_for_then_send` in `test_helper.rs` also uses these — update it

## Testing

- `make clean test` must pass locally
- All existing tests that use discovery should work with the new implementation
- No new tests needed — the existing integration tests validate the discovery flow
