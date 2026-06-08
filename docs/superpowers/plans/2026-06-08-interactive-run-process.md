# Interactive Run Process Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the debugger's `r` command to run a specific function or sub-flow interactively with user-supplied inputs and sandboxed execution.

**Architecture:** Add a `ProcessTarget` enum and extend `DebugCommand::RunReset` to carry an optional target and args. Add a `coerce_value` module for type heuristics. The CLI debugger prompts for inputs with pre-filled defaults; the GUI shows a slideout input panel. Execution requires init state, auto-resets after completion.

**Tech Stack:** Rust, serde_json, iced (GUI), rustyline (CLI)

---

### Task 1: Add `ProcessTarget` enum and extend `DebugCommand::RunReset`

**Files:**
- Modify: `flowcore/src/model/debug_command.rs`

- [ ] **Step 1: Add `ProcessTarget` enum**

Add above the `DebugCommand` enum:

```rust
/// Identifies a process (function or sub-flow) for the `run` command
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum ProcessTarget {
    /// A process identified by its numeric ID
    Id(usize),
    /// A process identified by its route path (e.g., "/flow/add")
    Route(String),
    /// A process identified by its name
    Name(String),
}
```

- [ ] **Step 2: Change `RunReset` variant to carry data**

Change:
```rust
RunReset,
```
To:
```rust
RunReset(Option<ProcessTarget>, Vec<String>),
```

Where `None` target means "run root flow" (preserving current behavior) and `Vec<String>` carries inline input args (empty vec when none provided).

- [ ] **Step 3: Fix all compilation errors from the variant change**

Every match on `RunReset` must now match `RunReset(_, _)` or `RunReset(target, args)`. Key locations:
- `flowr/src/lib/debugger.rs` lines 394-403: match arm for `RunReset`
- `flowr/src/lib/debug_client.rs` line 284: `Some(RunReset)` → parse target+args
- `flowr/src/bin/flowrgui/main.rs` line 2149: `DebugCommand::RunReset` → `DebugCommand::RunReset(None, vec![])`
- `flowcore/src/model/debug_command.rs` tests: `DebugCommand::RunReset` → `DebugCommand::RunReset(None, vec![])`

- [ ] **Step 4: Update CLI parsing to extract target and args**

In `debug_client.rs`, change the `"r" | "run" | "reset"` arm of `get_server_command()`:

```rust
"r" | "run" | "reset" => {
    match params {
        None => Some(RunReset(None, vec![])),
        Some(parts) if parts.is_empty() => Some(RunReset(None, vec![])),
        Some(parts) => {
            let target_str = &parts[0];
            let args = parts.get(1..).unwrap_or_default().to_vec();
            let target = if let Ok(id) = target_str.parse::<usize>() {
                ProcessTarget::Id(id)
            } else if target_str.starts_with('/') {
                ProcessTarget::Route(target_str.clone())
            } else {
                ProcessTarget::Name(target_str.clone())
            };
            Some(RunReset(Some(target), args))
        }
    }
}
```

Add `ProcessTarget` to the imports at top of file.

- [ ] **Step 5: Update help string**

In `debug_client.rs`, update the `'r'` line in `HELP_STRING`:

```
'r' | 'reset' or 'run' [target] [args]  - Reset/run: no args runs root flow,
                                 target can be function ID, /route, or name
                                 args are space-separated input values
```

- [ ] **Step 6: Run `make clippy` and `cargo fmt`**

Run: `cargo fmt && make clippy`

- [ ] **Step 7: Run tests**

Run: `make test`
Expected: All tests pass (behavior unchanged for `RunReset(None, vec![])`)

- [ ] **Step 8: Commit**

```bash
git add flowcore/src/model/debug_command.rs flowr/src/lib/debugger.rs \
  flowr/src/lib/debug_client.rs flowr/src/bin/flowrgui/main.rs
git commit -m "feat: extend RunReset command with optional process target and args (#2021)"
```

---

### Task 2: Add `Input::is_generic()` accessor

**Files:**
- Modify: `flowcore/src/model/input.rs`

- [ ] **Step 1: Add the accessor method**

In the `impl Input` block (after the `name()` method around line 171), add:

```rust
/// Return whether this input accepts generic types
#[must_use]
pub fn is_generic(&self) -> bool {
    self.generic
}
```

- [ ] **Step 2: Run `cargo fmt` and `make clippy`**

- [ ] **Step 3: Commit**

```bash
git add flowcore/src/model/input.rs
git commit -m "feat: add Input::is_generic() accessor for debugger input prompts (#2021)"
```

---

### Task 3: Add `coerce_value` module for type conversion

**Files:**
- Create: `flowr/src/lib/coerce_value.rs`
- Modify: `flowr/src/lib/lib.rs` (add `mod coerce_value;`)

- [ ] **Step 1: Write tests for the coercion module**

Create `flowr/src/lib/coerce_value.rs`:

```rust
use serde_json::Value;

use flowcore::errors::Result;

/// Coerce a string value using heuristic rules for generic inputs
pub(crate) fn coerce_generic(raw: &str) -> Value {
    let trimmed = raw.trim();

    if trimmed == "null" || trimmed == "Null" {
        return Value::Null;
    }

    if trimmed == "true" {
        return Value::Bool(true);
    }
    if trimmed == "false" {
        return Value::Bool(false);
    }

    // Quoted string: strip quotes, always string
    if trimmed.len() >= 2
        && trimmed.starts_with('"')
        && trimmed.ends_with('"')
    {
        return Value::String(trimmed[1..trimmed.len() - 1].to_string());
    }

    // Try number
    if let Ok(n) = trimmed.parse::<i64>() {
        return Value::Number(n.into());
    }
    if let Ok(n) = trimmed.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(n) {
            return Value::Number(num);
        }
    }

    // Try JSON array or object
    if (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || (trimmed.starts_with('{') && trimmed.ends_with('}'))
    {
        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            return val;
        }
    }

    // Fallback: plain string
    Value::String(trimmed.to_string())
}

/// Coerce a string to a Value, validating against an expected JSON type keyword.
/// `expected_type` is a hint like "Number", "String", "Bool", "Array", "Object", "Null".
/// Returns an error with a descriptive message if coercion fails.
pub(crate) fn coerce_typed(raw: &str, expected_type: &str, input_name: &str) -> Result<Value> {
    let value = coerce_generic(raw);
    let ok = match expected_type {
        "Number" => value.is_number(),
        "String" => value.is_string(),
        "Bool" => value.is_boolean(),
        "Array" => value.is_array(),
        "Object" => value.is_object(),
        "Null" => value.is_null(),
        _ => true, // unknown type, accept anything
    };
    if ok {
        Ok(value)
    } else {
        Err(format!(
            "Cannot coerce '{}' to {} for input '{}'",
            raw, expected_type, input_name
        ).into())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn coerce_bool_true() {
        assert_eq!(coerce_generic("true"), json!(true));
    }

    #[test]
    fn coerce_bool_false() {
        assert_eq!(coerce_generic("false"), json!(false));
    }

    #[test]
    fn coerce_null() {
        assert_eq!(coerce_generic("null"), Value::Null);
        assert_eq!(coerce_generic("Null"), Value::Null);
    }

    #[test]
    fn coerce_integer() {
        assert_eq!(coerce_generic("42"), json!(42));
        assert_eq!(coerce_generic("-7"), json!(-7));
    }

    #[test]
    fn coerce_float() {
        assert_eq!(coerce_generic("3.14"), json!(3.14));
    }

    #[test]
    fn coerce_quoted_string() {
        assert_eq!(coerce_generic("\"hello\""), json!("hello"));
        assert_eq!(coerce_generic("\"42\""), json!("42"));
    }

    #[test]
    fn coerce_unquoted_string() {
        assert_eq!(coerce_generic("hello"), json!("hello"));
    }

    #[test]
    fn coerce_array() {
        assert_eq!(coerce_generic("[1,2,3]"), json!([1, 2, 3]));
        assert_eq!(coerce_generic("[]"), json!([]));
    }

    #[test]
    fn coerce_object() {
        assert_eq!(coerce_generic("{\"a\":1}"), json!({"a": 1}));
    }

    #[test]
    fn coerce_typed_number_ok() {
        assert!(coerce_typed("42", "Number", "count").is_ok());
    }

    #[test]
    fn coerce_typed_number_fail() {
        let result = coerce_typed("hello", "Number", "count");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Cannot coerce"));
        assert!(msg.contains("Number"));
        assert!(msg.contains("count"));
    }

    #[test]
    fn coerce_typed_string_ok() {
        assert!(coerce_typed("hello", "String", "name").is_ok());
    }

    #[test]
    fn coerce_typed_unknown_type_passes() {
        assert!(coerce_typed("anything", "CustomType", "x").is_ok());
    }
}
```

- [ ] **Step 2: Add the module declaration**

In `flowr/src/lib/lib.rs`, add:

```rust
#[cfg(feature = "debugger")]
pub(crate) mod coerce_value;
```

- [ ] **Step 3: Run tests**

Run: `make test`
Expected: All coercion tests pass.

- [ ] **Step 4: Commit**

```bash
git add flowr/src/lib/coerce_value.rs flowr/src/lib/lib.rs
git commit -m "feat: add coerce_value module for input type conversion (#2021)"
```

---

### Task 4: Add target resolution in the debugger

**Files:**
- Modify: `flowr/src/lib/debugger.rs`

- [ ] **Step 1: Add resolve_target method**

Add a method to `Debugger` that resolves a `ProcessTarget` to a process ID:

```rust
/// Resolve a ProcessTarget to a function process_id
fn resolve_target(state: &RunState, target: &ProcessTarget) -> Result<usize> {
    match target {
        ProcessTarget::Id(id) => {
            if state.get_function(*id).is_some()
                || state.submission.manifest.flows().contains_key(id)
            {
                Ok(*id)
            } else {
                bail!("No process found matching '#{id}'")
            }
        }
        ProcessTarget::Route(route) => {
            Self::find_by_route(state, route)
                .ok_or_else(|| format!("No process found matching '{route}'").into())
        }
        ProcessTarget::Name(name) => {
            let matches: Vec<usize> = state
                .get_functions()
                .values()
                .filter(|f| f.name() == name)
                .map(|f| f.id())
                .collect();
            match matches.len() {
                0 => bail!("No process found matching '{name}'"),
                1 => Ok(matches[0]),
                _ => {
                    let mut msg = format!(
                        "Multiple processes match '{name}'. Use ID or route:\n"
                    );
                    for id in &matches {
                        if let Some(f) = state.get_function(*id) {
                            msg.push_str(&format!(
                                "  #{} @ '{}'\n",
                                f.id(),
                                f.route()
                            ));
                        }
                    }
                    bail!(msg)
                }
            }
        }
    }
}
```

Add `use crate::debug_command::ProcessTarget;` to imports.

- [ ] **Step 2: Update the `RunReset` match arm in `wait_for_command`**

Change the existing `Ok(RunReset)` arm (lines ~394-403) to:

```rust
Ok(RunReset(None, _)) => {
    return if state.get_number_of_jobs_created() > 0 {
        self.reset();
        self.debug_server.debugger_resetting();
        Ok((false, true))
    } else {
        self.debug_server.execution_starting();
        Ok((false, false))
    };
}
Ok(RunReset(Some(target), args)) => {
    if state.get_number_of_jobs_created() > 0 {
        self.debug_server.debugger_error(
            "Cannot run a process mid-execution. Reset first with 'r'.".into(),
        );
    } else {
        match Self::resolve_target(state, &target) {
            Ok(process_id) => {
                self.debug_server.message(format!(
                    "Resolved target to process #{process_id}"
                ));
                // TODO: Task 5 will add input gathering and execution here
            }
            Err(e) => self.debug_server.debugger_error(e.to_string()),
        }
    }
}
```

- [ ] **Step 3: Run `cargo fmt` and `make clippy`**

- [ ] **Step 4: Run tests**

Run: `make test`

- [ ] **Step 5: Commit**

```bash
git add flowr/src/lib/debugger.rs
git commit -m "feat: add process target resolution for run command (#2021)"
```

---

### Task 5: Implement CLI input gathering and execution

**Files:**
- Modify: `flowr/src/lib/debugger.rs`
- Modify: `flowr/src/lib/debugger_handler.rs`
- Modify: `flowr/src/lib/debug_client.rs` (CLI handler)
- Modify: `flowr/src/lib/debug_zmq_handler.rs` (ZMQ handler)
- Modify: `flowr/src/lib/debug_gui_handler.rs` (GUI channel handler)
- Modify: `flowr/src/lib/debug_server_message.rs` (new message variant)

This is the core task. The debugger needs to:
1. Look up the function's inputs (names, types, initializers)
2. Ask the handler to gather input values from the user
3. Coerce values
4. Fill the function's inputs
5. Trigger execution and capture outputs
6. Display outputs and reset

- [ ] **Step 1: Add `DebugServerMessage::GatherInputs` variant**

In `flowr/src/lib/debug_server_message.rs`, add a new variant to represent the input gathering request:

```rust
/// Request the client to gather input values for running a process
/// Contains: (process_id, Vec<(input_name, type_hint, default_value)>)
GatherInputs(usize, Vec<(String, String, Option<String>)>),
/// Display the output of a sandboxed process run
/// Contains: Vec<(output_route, value_string)>
ProcessOutput(Vec<(String, String)>),
```

- [ ] **Step 2: Add `gather_inputs` and `process_output` to `DebuggerHandler` trait**

In `debugger_handler.rs`, add:

```rust
/// Gather input values from the user for running a specific process.
/// `inputs` contains (name, type_hint, default_value) for each input.
/// Returns the user-provided values as strings, or None if cancelled.
fn gather_inputs(
    &mut self,
    process_id: usize,
    inputs: &[(String, String, Option<String>)],
) -> Option<Vec<String>>;

/// Display the outputs from a sandboxed process run
fn process_output(&mut self, outputs: &[(String, String)]);
```

- [ ] **Step 3: Implement `gather_inputs` in the CLI debug client**

In `debug_client.rs`, handle the new message in `process_server_message()`:

```rust
DebugServerMessage::GatherInputs(process_id, inputs) => {
    println!("Running process #{process_id}");
    let mut values = Vec::new();
    for (name, type_hint, default) in &inputs {
        let prompt = if type_hint.is_empty() {
            if let Some(def) = default {
                format!("{name} [{def}]: ")
            } else {
                format!("{name}: ")
            }
        } else if let Some(def) = default {
            format!("{name} ({type_hint}) [{def}]: ")
        } else {
            format!("{name} ({type_hint}): ")
        };

        match self.editor.readline(&prompt) {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    if let Some(def) = default {
                        values.push(def.clone());
                    } else {
                        values.push(String::new());
                    }
                } else {
                    values.push(line);
                }
            }
            Err(_) => {
                println!("Input cancelled");
                return Ok(Ack);
            }
        }
    }
    // Send the gathered values back as a RunReset command with the values
    // The debugger will use these to fill inputs and execute
    return Ok(DebugCommand::Modify(Some(
        std::iter::once(format!("__run_inputs:{process_id}"))
            .chain(values)
            .collect(),
    )));
}
```

Wait — this approach is awkward because `process_server_message` returns a `DebugCommand` to respond with. A cleaner approach: add the `gather_inputs` method to the `DebuggerHandler` trait so the debugger can call it directly during `wait_for_command`.

Let me revise. The `DebuggerHandler` trait already has `get_command()` which blocks waiting for user input. We can add `gather_inputs()` as a separate blocking call the debugger makes directly.

**Revised Step 3:** Implement in the ZMQ handler (`debug_zmq_handler.rs`) — this is what the CLI client actually talks to. The ZMQ handler sends a `GatherInputs` message to the client, and the client responds with the values.

Actually, looking at the architecture more carefully: the `DebuggerHandler` is called by the `Debugger` on the coordinator side. For the CLI flow:
- `DebugZmqHandler` implements `DebuggerHandler` on the coordinator side
- `DebugClient` is the separate process that talks over ZMQ

For the GUI:
- `DebugGuiHandler` implements `DebuggerHandler` using channels to the GUI in the same process

So `gather_inputs` on `DebuggerHandler` needs to:
- **ZMQ handler**: send a `GatherInputs` message, wait for client response
- **GUI handler**: send via channel, wait for channel response
- **CLI client**: handle `GatherInputs` message by prompting user, send values back

- [ ] **Step 3 (revised): Add `gather_inputs` and `process_output` to `DebuggerHandler` trait**

In `debugger_handler.rs`:

```rust
/// Gather input values from the user for running a specific process.
/// `inputs` contains (name, type_hint, default_value) for each input.
/// Returns the user-provided values as strings, or None if cancelled.
fn gather_inputs(
    &mut self,
    process_id: usize,
    inputs: &[(String, String, Option<String>)],
) -> Option<Vec<String>>;

/// Display the outputs from a sandboxed process run
fn process_output(&mut self, outputs: &[(String, String)]);
```

- [ ] **Step 4: Implement in ZMQ handler**

In `debug_zmq_handler.rs`, implement:

```rust
fn gather_inputs(
    &mut self,
    process_id: usize,
    inputs: &[(String, String, Option<String>)],
) -> Option<Vec<String>> {
    let msg = DebugServerMessage::GatherInputs(process_id, inputs.to_vec());
    self.send(&msg);
    // Wait for the client to respond with gathered values
    match self.receive() {
        Ok(DebugCommand::Modify(Some(values))) => {
            // First element is the marker, rest are values
            if values.first().map_or(false, |v| v.starts_with("__run_inputs:")) {
                Some(values.into_iter().skip(1).collect())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn process_output(&mut self, outputs: &[(String, String)]) {
    let msg = DebugServerMessage::ProcessOutput(outputs.to_vec());
    self.send(&msg);
}
```

- [ ] **Step 5: Implement in GUI handler**

In `debug_gui_handler.rs`, implement using the existing channel mechanism. The GUI will need a new message type for gathering inputs — but for now, return `None` (GUI panel will be added in Task 7):

```rust
fn gather_inputs(
    &mut self,
    _process_id: usize,
    _inputs: &[(String, String, Option<String>)],
) -> Option<Vec<String>> {
    // TODO: Task 7 will implement GUI panel for input gathering
    None
}

fn process_output(&mut self, outputs: &[(String, String)]) {
    let mut msg = String::from("Process output:\n");
    for (route, value) in outputs {
        msg.push_str(&format!("  {route}: {value}\n"));
    }
    self.send_message(msg);
}
```

- [ ] **Step 6: Handle `GatherInputs` and `ProcessOutput` in the CLI client**

In `debug_client.rs` `process_server_message()`, add handling for the new message variants:

```rust
DebugServerMessage::GatherInputs(process_id, inputs) => {
    println!("Running process #{process_id}");
    let mut values = Vec::new();
    for (name, type_hint, default) in &inputs {
        let prompt = if type_hint.is_empty() {
            if let Some(def) = default {
                format!("{name} [{def}]: ")
            } else {
                format!("{name}: ")
            }
        } else if let Some(def) = default {
            format!("{name} ({type_hint}) [{def}]: ")
        } else {
            format!("{name} ({type_hint}): ")
        };

        match self.editor.readline(&prompt) {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    values.push(default.clone().unwrap_or_default());
                } else {
                    values.push(line);
                }
            }
            Err(_) => {
                println!("Input cancelled");
                return Ok(Ack);
            }
        }
    }
    return Ok(DebugCommand::Modify(Some(
        std::iter::once(format!("__run_inputs:{process_id}")).chain(values).collect(),
    )));
}
DebugServerMessage::ProcessOutput(outputs) => {
    if outputs.is_empty() {
        println!("(no output)");
    } else {
        for (route, value) in outputs {
            println!("{route}: {value}");
        }
    }
}
```

- [ ] **Step 7: Implement the core run logic in debugger.rs**

In the `RunReset(Some(target), args)` arm (from Task 4), replace the TODO with the full logic:

```rust
Ok(RunReset(Some(target), args)) => {
    if state.get_number_of_jobs_created() > 0 {
        self.debug_server.debugger_error(
            "Cannot run a process mid-execution. Reset first with 'r'.".into(),
        );
    } else {
        match Self::resolve_target(state, &target) {
            Ok(process_id) => {
                if let Err(e) = self.run_process(state, process_id, &args) {
                    self.debug_server.debugger_error(e.to_string());
                }
            }
            Err(e) => self.debug_server.debugger_error(e.to_string()),
        }
    }
}
```

Add the `run_process` method:

```rust
fn run_process(
    &mut self,
    state: &mut RunState,
    process_id: usize,
    inline_args: &[String],
) -> Result<()> {
    let function = state
        .get_function(process_id)
        .ok_or("Could not get function")?
        .clone();

    let inputs = function.inputs();
    if inline_args.len() > inputs.len() {
        bail!(
            "Process has {} inputs but {} values were provided",
            inputs.len(),
            inline_args.len()
        );
    }

    // Build input descriptors: (name, type_hint, default_value)
    let input_descriptors: Vec<(String, String, Option<String>)> = inputs
        .iter()
        .enumerate()
        .map(|(i, input)| {
            let name = if input.name().is_empty() {
                format!("input_{i}")
            } else {
                input.name().to_string()
            };
            let type_hint = if input.is_generic() {
                String::new()
            } else {
                // For now use a generic type hint; could be refined later
                String::from("Value")
            };
            let default = if i < inline_args.len() {
                Some(inline_args[i].clone())
            } else {
                input.initializer().as_ref().map(|init| {
                    init.get_value().to_string()
                }).or_else(|| {
                    input.flow_initializer().as_ref().map(|init| {
                        init.get_value().to_string()
                    })
                })
            };
            (name, type_hint, default)
        })
        .collect();

    // Gather inputs from the user
    let Some(raw_values) = self.debug_server.gather_inputs(process_id, &input_descriptors)
    else {
        self.debug_server.message("Run cancelled".into());
        return Ok(());
    };

    if raw_values.len() != inputs.len() {
        bail!(
            "Expected {} input values but got {}",
            inputs.len(),
            raw_values.len()
        );
    }

    // Coerce values
    let mut coerced_values = Vec::new();
    for (i, raw) in raw_values.iter().enumerate() {
        let input = &inputs[i];
        let name = if input.name().is_empty() {
            format!("input_{i}")
        } else {
            input.name().to_string()
        };

        let value = if input.is_generic() {
            crate::coerce_value::coerce_generic(raw)
        } else {
            crate::coerce_value::coerce_typed(raw, "Value", &name)?
        };
        coerced_values.push(value);
    }

    // Fill the function's inputs with the coerced values
    let func = state
        .get_mut(process_id)
        .ok_or("Could not get mutable function")?;
    for (i, value) in coerced_values.into_iter().enumerate() {
        func.send(i, value)?;
    }

    // The function should now be ready to run — the coordinator will
    // create and dispatch the job on the next iteration.
    // Signal to continue execution, then reset after it completes.
    self.debug_server.execution_starting();
    return Ok(());
}
```

Note: `get_mut` is private to `RunState`. We need to either make it `pub(crate)` or add a public method. Check Task 5b.

- [ ] **Step 8: Make `RunState::get_mut` accessible**

In `flowr/src/lib/run_state.rs`, change `get_mut` visibility:

```rust
// Before:
fn get_mut(&mut self, id: usize) -> Option<&mut RuntimeFunction> {

// After:
#[cfg(feature = "debugger")]
pub(crate) fn get_mut(&mut self, id: usize) -> Option<&mut RuntimeFunction> {
```

Keep the non-debugger version as `fn get_mut` for existing callers.

- [ ] **Step 9: Also implement `DummyServer` stubs in test code**

In `debugger.rs` test module, add stubs to `DummyServer`:

```rust
fn gather_inputs(&mut self, _: usize, _: &[(String, String, Option<String>)]) -> Option<Vec<String>> {
    None
}
fn process_output(&mut self, _: &[(String, String)]) {}
```

Similarly in `coordinator.rs` test module's `DummyDebugServer`.

- [ ] **Step 10: Run `cargo fmt` and `make clippy`**

- [ ] **Step 11: Run tests**

Run: `make test`

- [ ] **Step 12: Commit**

```bash
git add flowr/src/lib/debugger.rs flowr/src/lib/debugger_handler.rs \
  flowr/src/lib/debug_client.rs flowr/src/lib/debug_zmq_handler.rs \
  flowr/src/lib/debug_gui_handler.rs flowr/src/lib/debug_server_message.rs \
  flowr/src/lib/run_state.rs
git commit -m "feat: implement CLI input gathering and process execution (#2021)"
```

---

### Task 6: Handle sub-flow execution

**Files:**
- Modify: `flowr/src/lib/debugger.rs`

Sub-flows are identified by being in the `flows()` map of the manifest. When the target is a sub-flow, we need to fill the entry-point inputs of all functions inside that sub-flow, then let the coordinator run until those functions complete.

- [ ] **Step 1: Extend `run_process` to detect sub-flows**

In the `run_process` method, after resolving the process_id, check if it's a flow:

```rust
let is_flow = state.submission.manifest.flows().contains_key(&process_id);

if is_flow {
    return self.run_sub_flow(state, process_id, inline_args);
}
```

- [ ] **Step 2: Implement `run_sub_flow`**

```rust
fn run_sub_flow(
    &mut self,
    state: &mut RunState,
    flow_id: usize,
    inline_args: &[String],
) -> Result<()> {
    // For a sub-flow, we need to identify the entry-point functions
    // (those that have inputs with initializers or that have inputs
    // connected from outside the flow).
    // For now, display a message that sub-flow execution is not yet supported.
    self.debug_server.message(
        "Sub-flow execution is not yet implemented. \
         Use a function ID or route to run individual functions.".into()
    );
    Ok(())
}
```

This is a placeholder — full sub-flow execution is complex and can be implemented incrementally. The core function execution path is the priority.

- [ ] **Step 3: Run `cargo fmt` and `make clippy`**

- [ ] **Step 4: Run tests**

Run: `make test`

- [ ] **Step 5: Commit**

```bash
git add flowr/src/lib/debugger.rs
git commit -m "feat: add sub-flow execution stub with informative message (#2021)"
```

---

### Task 7: Add GUI input panel (stub)

**Files:**
- Modify: `flowr/src/bin/flowrgui/main.rs`

The GUI needs a slideout panel for gathering inputs. This is a significant UI task. For the initial PR, we'll wire up the plumbing so the GUI handler responds to `gather_inputs` calls, with the actual panel implementation as a follow-up (potentially part of #2767).

- [ ] **Step 1: Update the GUI's `DebugReset` message handling**

In `main.rs`, the `Message::DebugReset` handler currently sends `DebugCommand::RunReset`. Update it to send `DebugCommand::RunReset(None, vec![])`.

- [ ] **Step 2: Update the GUI handler's `gather_inputs` to send a message**

For now, the GUI handler returns `None` (from Task 5 step 5), meaning the run is cancelled with a message. This is acceptable for the initial PR — the CLI is the primary interface.

A future PR can add the slideout panel with text fields for each input.

- [ ] **Step 3: Run `cargo fmt` and `make clippy`**

- [ ] **Step 4: Run tests**

Run: `make test`

- [ ] **Step 5: Commit**

```bash
git add flowr/src/bin/flowrgui/main.rs flowr/src/lib/debug_gui_handler.rs
git commit -m "feat: wire up GUI for RunReset with target, stub input panel (#2021)"
```

---

### Task 8: Final integration testing and PR

- [ ] **Step 1: Run full test suite**

Run: `make test`

- [ ] **Step 2: Run clippy**

Run: `make clippy`

- [ ] **Step 3: Run fmt**

Run: `cargo fmt`

- [ ] **Step 4: Push and create PR**

```bash
git push -u origin interactive_run_process_2021
gh pr create --title "Add interactive run process command to debugger (#2021)" \
  --body "## Summary
- Extended the \`r\` (run/reset) debugger command to accept an optional process target (by ID, route, or name) and inline input arguments
- Added type coercion module for converting user input strings to JSON values
- CLI debugger prompts for each input with pre-filled defaults from initializers
- GUI stub wired up (actual input panel is a follow-up)
- Sub-flow execution stub with informative message (follow-up for full implementation)

## Test plan
- [ ] Verify \`r\` with no args still runs/resets the root flow as before
- [ ] Verify \`r 5\` resolves function by ID and prompts for inputs
- [ ] Verify \`r /flow/add\` resolves by route
- [ ] Verify \`r add\` resolves by name, errors on ambiguity
- [ ] Verify inline args pre-fill the prompts
- [ ] Verify type coercion: numbers, bools, strings, null, arrays
- [ ] Verify error on mid-execution run attempt
- [ ] Verify existing tests pass (\`make test\`)
"
```

- [ ] **Step 5: Request code review**
