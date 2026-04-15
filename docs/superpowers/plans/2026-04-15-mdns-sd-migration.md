# Replace simpdiscover with mdns-sd Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `simpdiscover` crate with `mdns-sd` for service discovery, consolidating duplicated discovery code into a single `flowrlib::discovery` module.

**Architecture:** Create `flowr/src/lib/discovery.rs` with `discover_service()` and `enable_service_discovery()` functions using `mdns-sd`. All three binaries (flowrcli, flowrgui, flowrex) import from this shared module instead of duplicating discovery logic. The `discovery_port` parameter is removed since mdns-sd uses standard mDNS port 5353 automatically.

**Tech Stack:** Rust, mdns-sd 0.19, ZMQ (unchanged)

**Spec:** `docs/superpowers/specs/2026-04-15-mdns-sd-migration-design.md`

---

### File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `flowr/Cargo.toml` | Swap dependency |
| Create | `flowr/src/lib/discovery.rs` | Shared discovery functions |
| Modify | `flowr/src/lib/lib.rs` | Register new module |
| Modify | `flowr/src/lib/services.rs` | Move constants here, remove discovery port |
| Modify | `flowr/src/bin/flowrcli/cli/connections.rs` | Use shared discovery, update tests |
| Modify | `flowr/src/bin/flowrcli/cli/test_helper.rs` | Use shared discovery |
| Modify | `flowr/src/bin/flowrcli/main.rs` | Use shared discovery |
| Modify | `flowr/src/bin/flowrgui/gui/client_connection.rs` | Use shared discovery |
| Modify | `flowr/src/bin/flowrgui/gui/coordinator_connection.rs` | Use shared discovery |
| Modify | `flowr/src/bin/flowrgui/connection_manager.rs` | Use shared discovery |
| Modify | `flowr/src/bin/flowrex/main.rs` | Use shared discovery |

---

### Task 1: Create feature branch

**Files:** None

- [ ] **Step 1: Create and switch to feature branch**

```bash
git checkout -b mdns_sd_2025
```

- [ ] **Step 2: Verify clean state**

```bash
git status
```

Expected: clean working tree on `mdns_sd_2025` branch

---

### Task 2: Swap dependency in Cargo.toml

**Files:**
- Modify: `flowr/Cargo.toml:63` (simpdiscover line)

- [ ] **Step 1: Replace simpdiscover with mdns-sd**

In `flowr/Cargo.toml`, replace:
```toml
simpdiscover = "0.7"
```
with:
```toml
mdns-sd = { version = "0.19", default-features = false }
```

- [ ] **Step 2: Verify it compiles (will have unused dependency warning, that's fine)**

Run: `cargo check -p flowr 2>&1 | grep -E "error|warning.*mdns"`

Expected: No errors. May show "unused" warning for mdns-sd which is fine at this stage.

- [ ] **Step 3: Commit**

```bash
git add flowr/Cargo.toml Cargo.lock
git commit -m "Replace simpdiscover dependency with mdns-sd (#2025)"
```

---

### Task 3: Consolidate service name constants

**Files:**
- Modify: `flowr/src/lib/services.rs`

- [ ] **Step 1: Update services.rs**

The current file is:
```rust
/// `JOB_SERVICE_NAME` can be used to discover the queue serving jobs for execution
pub const JOB_SERVICE_NAME: &str = "jobs._flowr._tcp.local";

/// `RESULTS_JOB_SERVICE_NAME` can be used to discover the queue where to send job results
pub const RESULTS_JOB_SERVICE_NAME: &str = "results._flowr._tcp.local";

/// `CONTROL_SERVICE_NAME` is a control PUB/SUB socket used to control executors that
/// are listening on the `JOB_SERVICE` and sending results back via the `RESULTS_SERVICE`
pub const CONTROL_SERVICE_NAME: &str = "control._flowr._tcp.local";

/// This is the port for announcing and discovering the job queues
pub const JOB_QUEUES_DISCOVERY_PORT: u16 = 15003;
```

Replace the entire contents with:
```rust
/// The mDNS service type for flow services
pub const FLOW_SERVICE_TYPE: &str = "_flowr._tcp.local.";

/// `JOB_SERVICE_NAME` can be used to discover the queue serving jobs for execution
pub const JOB_SERVICE_NAME: &str = "jobs";

/// `RESULTS_JOB_SERVICE_NAME` can be used to discover the queue where to send job results
pub const RESULTS_JOB_SERVICE_NAME: &str = "results";

/// `CONTROL_SERVICE_NAME` is a control PUB/SUB socket used to control executors that
/// are listening on the `JOB_SERVICE` and sending results back via the `RESULTS_SERVICE`
pub const CONTROL_SERVICE_NAME: &str = "control";

/// Use this to discover the coordinator service by name
pub const COORDINATOR_SERVICE_NAME: &str = "runtime";

/// Use this to discover the debug service by name
#[cfg(feature = "debugger")]
pub const DEBUG_SERVICE_NAME: &str = "debug";
```

Note: mdns-sd separates the service type (`_flowr._tcp.local.`) from the instance name (`jobs`, `results`, etc.). The trailing dot on the service type is required by mDNS convention.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p flowr 2>&1 | grep error`

Expected: Compilation errors in files that import the old constants — that's expected at this stage. No errors within `services.rs` itself.

- [ ] **Step 3: Commit**

```bash
git add flowr/src/lib/services.rs
git commit -m "Consolidate service name constants, add FLOW_SERVICE_TYPE (#2025)"
```

---

### Task 4: Create the discovery module

**Files:**
- Create: `flowr/src/lib/discovery.rs`
- Modify: `flowr/src/lib/lib.rs`

- [ ] **Step 1: Create `flowr/src/lib/discovery.rs`**

```rust
//! Optional mDNS-SD service discovery helpers.
//!
//! These functions provide a convenient way to advertise and discover flow services
//! using mDNS-SD. They are not required — other binaries using `flowrlib` can
//! implement their own discovery mechanism.

use log::info;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use flowcore::errors::{Result, bail};

use crate::services::FLOW_SERVICE_TYPE;

/// Register a service for mDNS-SD discovery.
///
/// The returned `ServiceDaemon` must be kept alive by the caller — the service is
/// unregistered when the daemon is dropped.
pub fn enable_service_discovery(name: &str, service_port: u16) -> Result<ServiceDaemon> {
    let mdns = ServiceDaemon::new()
        .map_err(|e| format!("Could not create mDNS daemon: {e}"))?;

    let host_name = hostname::get()
        .unwrap_or_else(|_| "localhost".into())
        .to_string_lossy()
        .to_string();

    let service_hostname = format!("{host_name}.local.");

    let service_info = ServiceInfo::new(
        FLOW_SERVICE_TYPE,
        name,
        &service_hostname,
        "",
        service_port,
        None,
    )
    .map_err(|e| format!("Could not create mDNS ServiceInfo: {e}"))?
    .enable_addr_auto();

    mdns.register(service_info)
        .map_err(|e| format!("Could not register mDNS service: {e}"))?;

    info!("mDNS service registered: '{name}' on port {service_port}");

    Ok(mdns)
}

/// Discover a service by name using mDNS-SD. Blocks until the service is found.
///
/// Returns the service address as `"{ip}:{port}"`.
pub fn discover_service(name: &str) -> Result<String> {
    let mdns = ServiceDaemon::new()
        .map_err(|e| format!("Could not create mDNS daemon: {e}"))?;

    let receiver = mdns.browse(FLOW_SERVICE_TYPE)
        .map_err(|e| format!("Could not browse for mDNS services: {e}"))?;

    let full_name_suffix = format!(".{FLOW_SERVICE_TYPE}");

    loop {
        match receiver.recv() {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let instance = info.get_fullname()
                    .strip_suffix(&full_name_suffix)
                    .unwrap_or(info.get_fullname());

                if instance == name {
                    let port = info.get_port();
                    if let Some(addr) = info.get_addresses_v4().into_iter().next() {
                        let address = format!("{addr}:{port}");
                        info!("Discovered mDNS service '{name}' at {address}");
                        mdns.shutdown().ok();
                        return Ok(address);
                    }
                }
            }
            Err(e) => bail!("mDNS discovery error for '{name}': {e}"),
            _ => {} // Ignore other events (SearchStarted, ServiceFound, etc.)
        }
    }
}
```

- [ ] **Step 2: Add `hostname` dependency to Cargo.toml**

In `flowr/Cargo.toml`, add after the mdns-sd line:
```toml
hostname = "0.4"
```

- [ ] **Step 3: Register the module in `flowr/src/lib/lib.rs`**

Add after the `pub mod services;` line (around line 54):
```rust
/// Optional mDNS-SD service discovery helpers
pub mod discovery;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p flowr 2>&1 | grep "error" | grep -v "unused\|dead_code" | head -5`

Expected: Errors from other files that still import old constants — but `discovery.rs` itself should compile cleanly.

- [ ] **Step 5: Commit**

```bash
git add flowr/src/lib/discovery.rs flowr/src/lib/lib.rs flowr/Cargo.toml Cargo.lock
git commit -m "Add flowrlib::discovery module using mdns-sd (#2025)"
```

---

### Task 5: Update flowrcli connections

**Files:**
- Modify: `flowr/src/bin/flowrcli/cli/connections.rs:1-46`
- Modify: `flowr/src/bin/flowrcli/cli/test_helper.rs`

- [ ] **Step 1: Update connections.rs imports and remove local functions**

At the top of `flowr/src/bin/flowrcli/cli/connections.rs`, replace:
```rust
use simpdiscoverylib::{BeaconListener, BeaconSender};
```
with:
```rust
use flowrlib::discovery;
```

Remove these two functions (lines 25-46):
```rust
/// Try to discover a particular service by name
pub fn discover_service(discovery_port: u16, name: &str) -> Result<String> { ... }

/// Start a background thread that sends out beacons for service discovery by a client every second
pub fn enable_service_discovery(discovery_port: u16, name: &str, service_port: u16) -> Result<()> { ... }
```

Replace them with re-exports so callers within this binary don't need to change their import paths:
```rust
pub use flowrlib::discovery::discover_service;
pub use flowrlib::discovery::enable_service_discovery;
```

Remove the `COORDINATOR_SERVICE_NAME` and `DEBUG_SERVICE_NAME` constants (lines 19-23) since they now live in `flowrlib::services`:
```rust
pub use flowrlib::services::COORDINATOR_SERVICE_NAME;
#[cfg(feature = "debugger")]
pub use flowrlib::services::DEBUG_SERVICE_NAME;
```

- [ ] **Step 2: Update test_helper.rs**

In `flowr/src/bin/flowrcli/cli/test_helper.rs`, the imports on line 7-8 reference
`crate::cli::connections::{discover_service, enable_service_discovery, ...}`.
These will work via the re-exports. But the function signatures changed —
`discovery_port` is no longer a parameter.

Update `wait_for_then_send` (lines 21-23):
```rust
        // Old:
        let discovery_port = pick_unused_port().expect("No ports free");
        enable_service_discovery(discovery_port, "foo", test_port)
            .expect("Could not enable service discovery");
        // New:
        let _mdns = enable_service_discovery("foo", test_port)
            .expect("Could not enable service discovery");
```

And lines 29-30:
```rust
        // Old:
        let server_address =
            discover_service(discovery_port, "foo").expect("Could discovery service");
        // New:
        let server_address =
            discover_service("foo").expect("Could not discover service");
```

Remove the `pick_unused_port` import if no longer used for `discovery_port`:
Check if `pick_unused_port` is still used for `test_port` — yes it is (line 16), so keep the import.

- [ ] **Step 3: Update connections.rs tests**

In the test module at the bottom of `connections.rs`, update two tests:

**`coordinator_receive_wait_get_reply` (around line 257):**
```rust
    fn coordinator_receive_wait_get_reply() {
        let test_port = pick_unused_port().expect("No ports free");
        let mut coordinator_connection = CoordinatorConnection::new("test", test_port)
            .expect("Could not create CoordinatorConnection");

        let _mdns = enable_service_discovery("test", test_port)
            .expect("Could not enable service discovery");

        let coordinator_address =
            discover_service("test").expect("Could not discover service");
        // ... rest unchanged
```

**`coordinator_receive_nowait_get_reply` (around line 297):**
```rust
    fn coordinator_receive_nowait_get_reply() {
        let test_port = pick_unused_port().expect("No ports free");
        let mut coordinator_connection = CoordinatorConnection::new("test", test_port)
            .expect("Could not create CoordinatorConnection");
        let _mdns = enable_service_discovery("test", test_port)
            .expect("Could not enable service discovery");

        let coordinator_address =
            discover_service("test").expect("Could not discover service");
        // ... rest unchanged
```

Remove the unused `discovery_port` variable and its `pick_unused_port` call from both tests.

- [ ] **Step 4: Verify these tests compile**

Run: `cargo check -p flowr --bin flowrcli 2>&1 | grep error | head -10`

Expected: May have errors from other files still using old API. The connections module and tests themselves should compile.

- [ ] **Step 5: Commit**

```bash
git add flowr/src/bin/flowrcli/cli/connections.rs flowr/src/bin/flowrcli/cli/test_helper.rs
git commit -m "Update flowrcli connections to use flowrlib::discovery (#2025)"
```

---

### Task 6: Update flowrcli main.rs

**Files:**
- Modify: `flowr/src/bin/flowrcli/main.rs`

- [ ] **Step 1: Update imports**

Remove:
```rust
use simpdiscoverylib::BeaconListener;
```
(This may have already been removed if it was only used by the local `discover_service` function.)

In the services import (around line 28-30), change:
```rust
use flowrlib::services::{
    CONTROL_SERVICE_NAME, JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME,
};
```
to:
```rust
use flowrlib::discovery::enable_service_discovery;
use flowrlib::services::{
    CONTROL_SERVICE_NAME, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME,
};
```

- [ ] **Step 2: Update the coordinator function**

In the `coordinator()` function (around lines 289-295), change:
```rust
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, ports.0)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, RESULTS_JOB_SERVICE_NAME, ports.2)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, CONTROL_SERVICE_NAME, ports.3)?;
```
to:
```rust
    let _mdns_jobs = enable_service_discovery(JOB_SERVICE_NAME, ports.0)?;
    let _mdns_results = enable_service_discovery(RESULTS_JOB_SERVICE_NAME, ports.2)?;
    let _mdns_control = enable_service_discovery(CONTROL_SERVICE_NAME, ports.3)?;
```

The `_mdns_*` handles keep the ServiceDaemons alive for the duration of the function.

- [ ] **Step 3: Remove the local `discover_service` function**

If `flowrcli/main.rs` has a local `discover_service` function (it does for server mode), remove it and import from `flowrlib::discovery` instead. Check if it's used — search for `discover_service` calls in the file.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p flowr --bin flowrcli 2>&1 | grep error | head -5`

Expected: No errors in flowrcli.

- [ ] **Step 5: Commit**

```bash
git add flowr/src/bin/flowrcli/main.rs
git commit -m "Update flowrcli main to use flowrlib::discovery (#2025)"
```

---

### Task 7: Update flowrgui

**Files:**
- Modify: `flowr/src/bin/flowrgui/gui/client_connection.rs`
- Modify: `flowr/src/bin/flowrgui/gui/coordinator_connection.rs`
- Modify: `flowr/src/bin/flowrgui/connection_manager.rs`

- [ ] **Step 1: Update client_connection.rs**

Replace:
```rust
use simpdiscoverylib::BeaconListener;
```
with nothing (remove the import).

Remove the local `discover_service` function (lines 13-18).

Add a re-export at the top:
```rust
pub use flowrlib::discovery::discover_service;
```

- [ ] **Step 2: Update coordinator_connection.rs**

Replace:
```rust
use simpdiscoverylib::BeaconSender;
```
with nothing (remove the import).

Remove the local `enable_service_discovery` function (lines 24-37).

Remove the local constants `COORDINATOR_SERVICE_NAME` and `DEBUG_SERVICE_NAME` (lines 19-22).

Add re-exports:
```rust
pub use flowrlib::discovery::enable_service_discovery;
pub use flowrlib::services::COORDINATOR_SERVICE_NAME;
pub use flowrlib::services::DEBUG_SERVICE_NAME;
```

- [ ] **Step 3: Update connection_manager.rs**

In the imports, change `JOB_QUEUES_DISCOVERY_PORT` usage. Around lines 175-178, change:
```rust
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, ports.0)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, RESULTS_JOB_SERVICE_NAME, ports.2)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, CONTROL_SERVICE_NAME, ports.3)?;
```
to:
```rust
    let _mdns_jobs = enable_service_discovery(JOB_SERVICE_NAME, ports.0)?;
    let _mdns_results = enable_service_discovery(RESULTS_JOB_SERVICE_NAME, ports.2)?;
    let _mdns_control = enable_service_discovery(CONTROL_SERVICE_NAME, ports.3)?;
```

Update imports to remove `JOB_QUEUES_DISCOVERY_PORT` and add `enable_service_discovery` from the re-export or directly from `flowrlib::discovery`.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p flowr --bin flowrgui 2>&1 | grep error | head -5`

Expected: No errors in flowrgui.

- [ ] **Step 5: Commit**

```bash
git add flowr/src/bin/flowrgui/gui/client_connection.rs flowr/src/bin/flowrgui/gui/coordinator_connection.rs flowr/src/bin/flowrgui/connection_manager.rs
git commit -m "Update flowrgui to use flowrlib::discovery (#2025)"
```

---

### Task 8: Update flowrex

**Files:**
- Modify: `flowr/src/bin/flowrex/main.rs`

- [ ] **Step 1: Update imports**

Replace:
```rust
use simpdiscoverylib::BeaconListener;
```

And in the services import, remove `JOB_QUEUES_DISCOVERY_PORT`:
```rust
use flowrlib::services::{
    CONTROL_SERVICE_NAME, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME,
};
```

Add:
```rust
use flowrlib::discovery::discover_service;
```

- [ ] **Step 2: Remove local discover_service function**

Remove the local `discover_service` function (lines 37-42).

- [ ] **Step 3: Update the three discover calls**

Around lines 107-118, change:
```rust
        let job_service = format!(
            "tcp://{}",
            discover_service(JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME)?
        );
        let results_service = format!(
            "tcp://{}",
            discover_service(JOB_QUEUES_DISCOVERY_PORT, RESULTS_JOB_SERVICE_NAME)?
        );
        let control_service = format!(
            "tcp://{}",
            discover_service(JOB_QUEUES_DISCOVERY_PORT, CONTROL_SERVICE_NAME)?
        );
```
to:
```rust
        let job_service = format!(
            "tcp://{}",
            discover_service(JOB_SERVICE_NAME)?
        );
        let results_service = format!(
            "tcp://{}",
            discover_service(RESULTS_JOB_SERVICE_NAME)?
        );
        let control_service = format!(
            "tcp://{}",
            discover_service(CONTROL_SERVICE_NAME)?
        );
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p flowr --bin flowrex 2>&1 | grep error | head -5`

Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add flowr/src/bin/flowrex/main.rs
git commit -m "Update flowrex to use flowrlib::discovery (#2025)"
```

---

### Task 9: Clean build and test

**Files:** None (verification only)

- [ ] **Step 1: Run clippy**

Run: `make clippy 2>&1 | tail -5`

Expected: No errors. Fix any warnings about unused imports or variables.

- [ ] **Step 2: Run cargo fmt**

Run: `cargo fmt`

- [ ] **Step 3: Run full test suite**

Run: `make clean test 2>&1 | tail -30`

Expected: All tests pass, zero failures. The only ignored tests should be the two pre-existing ones (sequence-of-sequences and array_initializers).

- [ ] **Step 4: Fix any issues found and re-run**

If any tests fail, investigate and fix. Common issues:
- Service name format may need adjustment (trailing dots, underscores)
- `discover_service` may need a timeout to avoid hanging forever
- `ServiceDaemon` may need to be kept alive longer

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "Fix issues found during testing (#2025)"
```

---

### Task 10: Create PR and monitor CI

**Files:** None

- [ ] **Step 1: Push branch**

```bash
git push -u origin mdns_sd_2025
```

- [ ] **Step 2: Create PR**

```bash
gh pr create --title "Replace simpdiscover with mdns-sd (#2025)" --body "$(cat <<'EOF'
## Summary
- Replace `simpdiscover` with `mdns-sd` for mDNS-SD standard service discovery
- Consolidate duplicated discovery code into `flowrlib::discovery` module
- Move service name constants to `flowrlib::services`
- Remove custom discovery port (uses standard mDNS port 5353)

Fixes #2025

## Test plan
- [x] `make clean test` passes locally
- [x] `make clippy` passes
- [x] `cargo fmt` applied
- [ ] CI passes on all runners

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 3: Monitor CI**

Wait for CI to complete on all runners (macos-14, ubuntu-24.04, ubuntu-24.04-arm).
