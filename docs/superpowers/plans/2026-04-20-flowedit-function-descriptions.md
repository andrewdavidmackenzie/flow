# flowedit: Show Function/Flow Descriptions — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Display function/flow descriptions in flowedit tooltips and allow editing them, using a reference-based architecture backed by canonical definitions from flowclib.

**Architecture:** Add `description` field to `FlowDefinition`. Replace flowedit's custom parsing/scanning with flowclib functions (`parser::parse()`, `LibraryManifest::load()`). UI structs hold references to canonical `FunctionDefinition`/`FlowDefinition` objects — no copying. Libraries are lazily loaded when first referenced by the flow. Descriptions are read from the referenced definitions for tooltips, mutated via mutable references for editing, and serialized back via flowclib on save.

**Tech Stack:** Rust, iced 0.14.0, flowcore model types, flowrclib parser, serde TOML/JSON

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `flowcore/src/model/flow_definition.rs` | Modify | Add `description` field |
| `flowedit/src/library_panel.rs` | Rewrite | Replace filesystem scanning with references to cached `LibraryManifest` + parsed definitions |
| `flowedit/src/canvas_view.rs` | Modify | `NodeLayout` references definitions for description; two-zone hit testing |
| `flowedit/src/main.rs` | Modify | Lazy library loading, shared definition cache, viewer references definitions, description editing, serialization on save |

---

### Task 1: Add `description` field to `FlowDefinition`

**Files:**
- Modify: `flowcore/src/model/flow_definition.rs:35-80` (struct + Default impl)

- [ ] **Step 1: Write failing test for `FlowDefinition` deserialization with description**

Add this test at the end of the existing `mod test` block in `flowcore/src/model/flow_definition.rs` (after line ~540):

```rust
#[test]
fn deserialize_with_description() {
    use url::Url;
    use crate::deserializers::deserializer::get;

    let toml_str = r#"
    flow = "described_flow"
    description = "A flow that does something useful"
    "#;

    let url = Url::parse("file:///fake.toml").expect("Could not parse URL");
    let deserializer = get::<FlowDefinition>(&url).expect("Could not get deserializer");
    let flow: FlowDefinition = deserializer
        .deserialize(toml_str, Some(&url))
        .expect("Could not deserialize FlowDefinition with description");
    assert_eq!(flow.description, "A flow that does something useful");
}

#[test]
fn deserialize_without_description() {
    use url::Url;
    use crate::deserializers::deserializer::get;

    let toml_str = r#"
    flow = "no_desc_flow"
    "#;

    let url = Url::parse("file:///fake.toml").expect("Could not parse URL");
    let deserializer = get::<FlowDefinition>(&url).expect("Could not get deserializer");
    let flow: FlowDefinition = deserializer
        .deserialize(toml_str, Some(&url))
        .expect("Could not deserialize FlowDefinition without description");
    assert_eq!(flow.description, "");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p flowcore deserialize_with_description`
Expected: FAIL — `FlowDefinition` has no field `description`

- [ ] **Step 3: Add `description` field to `FlowDefinition` struct and Default impl**

In `flowcore/src/model/flow_definition.rs`, add the field to the struct (after the `docs` field, around line 56):

```rust
    /// Optional description of what this flow does
    #[serde(default)]
    pub description: String,
```

And add to the `Default` impl (around line 139, after `docs: String::new(),`):

```rust
            description: String::new(),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p flowcore deserialize_with_description deserialize_without_description`
Expected: PASS

- [ ] **Step 5: Run full flowcore tests**

Run: `cargo test -p flowcore`
Expected: PASS — existing tests still work since `description` has `#[serde(default)]`

- [ ] **Step 6: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: No warnings or errors

- [ ] **Step 7: Commit**

```bash
git add flowcore/src/model/flow_definition.rs
git commit -m "Add optional description field to FlowDefinition (#2574)"
```

---

### Task 2: Replace flow loading with `parser::parse()`

Currently `main.rs` has `load_flow()` and `resolve_single_function_ports()` which manually deserialize TOML files. Replace these with `flowrclib::compiler::parser::parse()` which returns a fully resolved `FlowDefinition` with `subprocesses` populated.

**Files:**
- Modify: `flowedit/src/main.rs` (load_flow, resolve_single_function_ports, add_library_function)

- [ ] **Step 1: Replace `load_flow()` to use `parser::parse()`**

In `flowedit/src/main.rs`, replace the body of `load_flow()` (around line 3254). Instead of manually reading the file, deserializing, and calling `resolve_single_function_ports()` for each process reference, call:

```rust
use flowrclib::compiler::parser;

fn load_flow(
    path: &PathBuf,
) -> Result<(String, Vec<NodeLayout>, Vec<EdgeLayout>, FlowDefinition), String> {
    let abs_path = if path.is_absolute() {
        path.clone()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.clone())
    };

    let url = Url::from_file_path(&abs_path)
        .map_err(|()| format!("Could not create URL from path: {}", abs_path.display()))?;

    let provider = build_meta_provider();

    let process = parser::parse(&url, &provider)
        .map_err(|e| format!("Could not parse flow: {e}"))?;

    match process {
        Process::FlowProcess(flow) => {
            let flow_name = flow.name.clone();
            let (nodes, edges) = build_layouts_from_flow(&flow);
            Ok((flow_name, nodes, edges, flow))
        }
        Process::FunctionProcess(_) => {
            Err("Expected a flow definition, got a function definition".to_string())
        }
    }
}
```

Write a helper `build_layouts_from_flow()` that creates `NodeLayout` and `EdgeLayout` vectors from the parsed `FlowDefinition`. Each `NodeLayout` should store a reference (or key) back to the subprocess definition rather than copying fields. For now, store the subprocess name as a key to look up the definition from `flow.subprocesses`.

- [ ] **Step 2: Remove `resolve_single_function_ports()`**

This function manually deserializes TOML — it's no longer needed since `parser::parse()` fully resolves all subprocesses. Remove it and all call sites.

- [ ] **Step 3: Update `add_library_function()` to use the parsed flow's subprocess definitions**

When adding a library function to the canvas, the definition should already be available in the flow's resolved subprocesses (or can be obtained via `parser::parse()` on the lib:// URL). The ports and description come from the canonical definition, not from re-parsing.

- [ ] **Step 4: Verify build compiles**

Run: `cargo build -p flowedit`
Expected: Compiles

- [ ] **Step 5: Run tests**

Run: `cargo test -p flowedit`
Expected: PASS

- [ ] **Step 6: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: No warnings or errors

- [ ] **Step 7: Commit**

```bash
git add flowedit/src/main.rs
git commit -m "Replace custom flow parsing with flowrclib parser::parse() (#2574)"
```

---

### Task 3: Lazy library loading and shared definition cache

When the parsed flow has `lib_references` or `context_references`, load the corresponding `LibraryManifest` on first encounter and cache it. The library panel displays from these cached manifests — no filesystem scanning.

**Files:**
- Modify: `flowedit/src/main.rs` (FlowEdit struct, initialization)
- Rewrite: `flowedit/src/library_panel.rs` (remove filesystem scanning, reference cached manifests)

- [ ] **Step 1: Add library cache to `FlowEdit`**

In `flowedit/src/main.rs`, add a library cache to the `FlowEdit` struct (around line 245):

```rust
use std::sync::{Arc, RwLock};
use flowcore::model::lib_manifest::LibraryManifest;

struct FlowEdit {
    windows: HashMap<window::Id, WindowState>,
    root_window: Option<window::Id>,
    focused_window: Option<window::Id>,
    /// Cached library manifests, keyed by library URL
    library_cache: HashMap<Url, LibraryManifest>,
    /// Parsed definitions from libraries, keyed by lib:// URL
    lib_definitions: HashMap<Url, Process>,
    root_flow_path: Option<PathBuf>,
    show_lib_paths: bool,
    lib_paths: Vec<String>,
}
```

- [ ] **Step 2: Populate cache after flow is loaded**

After `parser::parse()` returns the flow, iterate `flow.lib_references` and `flow.context_references`. For each unique library, call `LibraryManifest::load()` and cache the result. Then for each locator in the manifest, call `parser::parse()` to get the definition and cache it in `lib_definitions`.

```rust
fn load_libraries(
    lib_refs: &BTreeSet<Url>,
    provider: &dyn Provider,
    cache: &mut HashMap<Url, LibraryManifest>,
    definitions: &mut HashMap<Url, Process>,
) {
    for lib_ref in lib_refs {
        // Extract library root URL (e.g., lib://flowstdlib from lib://flowstdlib/math/add)
        let lib_root = // ... extract library root from lib_ref
        if cache.contains_key(&lib_root) {
            continue; // Already loaded
        }

        let arc_provider = Arc::new(/* provider */);
        if let Ok((manifest, _)) = LibraryManifest::load(&arc_provider, &lib_root) {
            for (func_url, _locator) in &manifest.locators {
                if let Ok(process) = parser::parse(func_url, provider) {
                    definitions.insert(func_url.clone(), process);
                }
            }
            cache.insert(lib_root, manifest);
        }
    }
}
```

- [ ] **Step 3: Rewrite `library_panel.rs` to display from cached definitions**

Replace the entire filesystem scanning approach. The `LibraryTree` no longer scans — it receives references to the cached manifests and definitions. Build the tree structure from manifest locator URLs (extracting library/category/function names from the URL hierarchy).

Remove `scan_functions()`, `scan_context_functions()`, `scan_categories()`, `resolve_lib_path()` and all filesystem code. The `FunctionEntry` struct reads name, source, and description from the referenced definition — no copied fields needed.

- [ ] **Step 4: Test with line-echo (no lib references)**

Run: `cargo run -p flowedit -- examples/line-echo/line-echo.toml`
Expected: Library panel is empty (or shows only context functions if referenced). No library manifests loaded.

- [ ] **Step 5: Test with fibonacci (has lib references)**

Run: `cargo run -p flowedit -- examples/fibonacci/fibonacci.toml`
Expected: Library panel shows flowstdlib functions referenced by the flow. Hover tooltips show descriptions.

- [ ] **Step 6: Run tests**

Run: `cargo test -p flowedit`
Expected: PASS (update/remove tests that relied on filesystem scanning)

- [ ] **Step 7: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: No warnings or errors

- [ ] **Step 8: Commit**

```bash
git add flowedit/src/main.rs flowedit/src/library_panel.rs
git commit -m "Replace filesystem scanning with lazy library loading from manifests (#2574)"
```

---

### Task 4: Add library manually via button

The library panel only shows libraries referenced by the current flow. Add a button (e.g., "+ Library") that lets the user browse to a library root or enter a `lib://` URL, loads its manifest, and adds it to the cache — making its functions available to drag onto the canvas.

**Files:**
- Modify: `flowedit/src/library_panel.rs` (add button to panel)
- Modify: `flowedit/src/main.rs` (message handling, library loading)

- [ ] **Step 1: Add `AddLibrary` message variant**

In `LibraryMessage`:

```rust
    /// User requested to add a new library
    AddLibrary,
```

And in `LibraryAction`:

```rust
    AddLibrary,
```

- [ ] **Step 2: Add "+ Library" button to panel header**

In the `view()` method of `LibraryTree`, add a button near the "Process Library" header:

```rust
let add_lib_btn = button(text("+ Library").size(11))
    .on_press(LibraryMessage::AddLibrary)
    .style(button::secondary)
    .padding([2, 6]);

content = content.push(
    Row::new()
        .spacing(8)
        .push(header)
        .push(add_lib_btn)
);
```

- [ ] **Step 3: Handle `AddLibrary` in `main.rs`**

When the user clicks "+ Library", open a file dialog (using `rfd`) to browse to a library directory. Then load its manifest via `LibraryManifest::load()`, parse its definitions, add to the cache, and refresh the library panel.

- [ ] **Step 4: Verify build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`
Expected: Compiles and passes

- [ ] **Step 5: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: Clean

- [ ] **Step 6: Commit**

```bash
git add flowedit/src/library_panel.rs flowedit/src/main.rs
git commit -m "Add button to manually load a library into the panel (#2574)"
```

---

### Task 5: Library panel tooltips

With the library panel now referencing cached definitions, add hover tooltips showing the description.

**Files:**
- Modify: `flowedit/src/library_panel.rs` (view function)

- [ ] **Step 1: Add iced `Tooltip` import**

```rust
use iced::widget::{button, container, scrollable, text, tooltip, Column, Row};
```

- [ ] **Step 2: Wrap function entries in `Tooltip` when description is non-empty**

In the `view()` method, when rendering each function entry, wrap the row in an iced `Tooltip` widget if the referenced definition has a non-empty description:

```rust
let entry_widget: Element<'_, LibraryMessage> =
    if description.is_empty() {
        row.into()
    } else {
        tooltip(
            row,
            text(&description).size(11),
            tooltip::Position::Bottom,
        )
        .gap(2)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(
                iced::Color::from_rgb(0.12, 0.12, 0.12),
            )),
            border: iced::Border {
                color: iced::Color::WHITE,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
    };
```

The `description` is read directly from the referenced definition — no field on `FunctionEntry`.

- [ ] **Step 3: Verify build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`
Expected: Compiles and passes

- [ ] **Step 4: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: Clean

- [ ] **Step 5: Commit**

```bash
git add flowedit/src/library_panel.rs
git commit -m "Add hover tooltips showing descriptions in library panel (#2574)"
```

---

### Task 6: Two-zone canvas tooltips

**Files:**
- Modify: `flowedit/src/canvas_view.rs` (hover detection, hit testing)

- [ ] **Step 1: Write test for source text zone hit testing**

Add to `mod test` in `flowedit/src/canvas_view.rs`:

```rust
#[test]
fn hit_test_source_text_zone() {
    let node = NodeLayout {
        alias: "test".into(),
        source: "lib://flowstdlib/math/add".into(),
        x: 100.0,
        y: 100.0,
        width: 180.0,
        height: 120.0,
        inputs: vec![],
        outputs: vec![],
        initializers: HashMap::new(),
    };
    // Source text is centered at (node.x + width/2, node.y + 34.0)
    let source_center = Point::new(190.0, 134.0);
    assert!(is_in_source_text_zone(&node, source_center));
    // Point clearly outside source text zone but inside node
    let node_body = Point::new(110.0, 200.0);
    assert!(!is_in_source_text_zone(&node, node_body));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p flowedit hit_test_source_text_zone`
Expected: FAIL — `is_in_source_text_zone` does not exist

- [ ] **Step 3: Add `is_in_source_text_zone` function**

```rust
fn is_in_source_text_zone(node: &NodeLayout, point: Point) -> bool {
    let text_center_x = node.x + node.width / 2.0;
    let text_top_y = node.y + 34.0;
    let text_height = SOURCE_FONT_SIZE + 4.0;
    let text_half_width = node.width * 0.4;

    point.x >= text_center_x - text_half_width
        && point.x <= text_center_x + text_half_width
        && point.y >= text_top_y
        && point.y <= text_top_y + text_height
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p flowedit hit_test_source_text_zone`
Expected: PASS

- [ ] **Step 5: Modify hover detection for two zones**

Replace the hover tracking block in `canvas_view.rs` (around line 1456-1474). The node's description is looked up from the referenced definition (via the subprocess name/key stored in `NodeLayout`):

- Inner zone (source text box): show full source path tooltip — always, not just when truncated
- Outer zone (rest of node body): show description from referenced definition — only if non-empty
- Inner zone wins over outer zone

```rust
                    // Track hover for two-zone node tooltip
                    let new_hover = hit_test_node(self.nodes, world_pos);
                    if new_hover != state.hover_node || new_hover.is_some() {
                        state.hover_node = new_hover;
                        let tooltip_data = new_hover
                            .and_then(|idx| self.nodes.get(idx))
                            .and_then(|n| {
                                let bottom_center = transform_point(
                                    Point::new(n.x + n.width / 2.0, n.y + n.height),
                                    zoom,
                                    offset,
                                );
                                if is_in_source_text_zone(n, world_pos) {
                                    Some((n.source.clone(), bottom_center.x, bottom_center.y))
                                } else {
                                    // Look up description from referenced definition
                                    let desc = // ... get description from definition reference
                                    if !desc.is_empty() {
                                        Some((desc.to_string(), bottom_center.x, bottom_center.y))
                                    } else {
                                        None
                                    }
                                }
                            });
                        return Some(canvas::Action::publish(CanvasMessage::HoverChanged(
                            tooltip_data,
                        )));
                    }
```

The exact mechanism for looking up the description depends on how `NodeLayout` references its definition (resolved in Task 2).

- [ ] **Step 6: Run tests**

Run: `cargo test -p flowedit`
Expected: PASS

- [ ] **Step 7: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: Clean

- [ ] **Step 8: Commit**

```bash
git add flowedit/src/canvas_view.rs
git commit -m "Add two-zone canvas tooltips: source text vs description (#2574)"
```

---

### Task 7: Editable description in function viewer

The `FunctionViewer` references the canonical definition. Editing the description mutates the definition directly via a mutable reference.

**Files:**
- Modify: `flowedit/src/main.rs` (Message enum, FunctionViewer, update handler, view rendering)

- [ ] **Step 1: Add `FunctionDescriptionChanged` message variant**

In the `Message` enum (after `FunctionNameChanged`):

```rust
    /// Function description edited
    FunctionDescriptionChanged(window::Id, String),
```

- [ ] **Step 2: Modify `FunctionViewer` to reference the canonical definition**

Instead of storing copied `name`, `description`, etc., `FunctionViewer` holds a reference (or key) to the canonical `FunctionDefinition` or `FlowDefinition`. Reads and writes go through this reference.

- [ ] **Step 3: Add update handler for description changes**

```rust
            Message::FunctionDescriptionChanged(win_id, new_desc) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    // Mutate the canonical definition's description directly
                    // via the reference held by the viewer
                    win.unsaved_edits += 1;
                }
            }
```

The mutation goes to the canonical definition, so palette tooltips and canvas tooltips see the change immediately on next render — no sync needed.

- [ ] **Step 4: Add description `text_input` to viewer rendering**

Below the name input, add a description input nearly full width:

```rust
                let desc_input = container(
                    text_input("Description", &description)
                        .on_input(move |s| Message::FunctionDescriptionChanged(window_id, s))
                        .size(13)
                        .padding(6)
                        .width(480),
                )
                .center_x(Fill);
```

For library and context functions, omit the `.on_input()` to make the field read-only. Only provided implementations get editable descriptions.

- [ ] **Step 5: Verify build and tests**

Run: `cargo build -p flowedit && cargo test -p flowedit`
Expected: Compiles and passes

- [ ] **Step 6: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: Clean

- [ ] **Step 7: Commit**

```bash
git add flowedit/src/main.rs
git commit -m "Add editable description field to function viewer (#2574)"
```

---

### Task 8: Serialize flow on save using flowclib

When saving the flow, use flowclib/flowcore serialization to write the `FlowDefinition` (including any edited descriptions) back to TOML. No custom serialization code in flowedit.

**Files:**
- Modify: `flowedit/src/main.rs` (save function)

- [ ] **Step 1: Replace custom save logic with flowcore serialization**

The `FlowDefinition` struct derives `Serialize`, so it can be serialized to TOML directly via serde. The edited description (mutated in the canonical definition) is automatically included.

Find the existing save function in `main.rs` and ensure it serializes the canonical `FlowDefinition` using serde_toml (or the appropriate flowcore serializer), not by manually constructing TOML strings.

- [ ] **Step 2: Test save and reload**

- Open a flow with `cargo run -p flowedit`
- Edit a description on a provided implementation
- Save
- Verify the TOML file contains the `description` field
- Reopen — verify the description persists

- [ ] **Step 3: Run tests**

Run: `make test`
Expected: PASS

- [ ] **Step 4: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: Clean

- [ ] **Step 5: Commit**

```bash
git add flowedit/src/main.rs
git commit -m "Use flowcore serialization for flow save with descriptions (#2574)"
```

---

### Task 9: Manual testing and final verification

- [ ] **Step 1: Run full `make test`**

Run: `make test`
Expected: All tests pass

- [ ] **Step 2: Run `make clippy` and `cargo fmt`**

Run: `make clippy && cargo fmt`
Expected: Clean

- [ ] **Step 3: Test with line-echo (no lib references)**

Run: `cargo run -p flowedit -- examples/line-echo/line-echo.toml`
Verify: Library panel shows no library functions (only context if referenced). No manifests loaded.

- [ ] **Step 4: Test with fibonacci (has lib references)**

Run: `cargo run -p flowedit -- examples/fibonacci/fibonacci.toml`
Verify:
- Library panel shows flowstdlib functions used by the flow
- Hover over functions in palette — tooltip shows description
- Hover over source text on canvas node — full source path tooltip
- Hover over rest of node — description tooltip
- Edit description in viewer — tooltips update immediately
- Save and reload — description persists

- [ ] **Step 5: Final commit if any adjustments were needed**

```bash
git add -A
git commit -m "Polish description tooltips after manual testing (#2574)"
```
