# flowedit: Show Function/Flow Descriptions — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Display function/flow descriptions in flowedit tooltips and allow editing them, backed by a shared-ownership data architecture.

**Architecture:** Add `description` field to `FlowDefinition` (mirroring `FunctionDefinition`). Extend `LibraryManifest` to hold parsed `Process` definitions alongside locators. Parse descriptions from TOML during library scanning. Add two-zone canvas tooltips (source text inner box vs description outer box). Add editable description field to function viewer.

**Note on shared-ownership refactor:** The spec describes a full `Arc<RwLock<>>` shared-ownership architecture where UI structs reference canonical definitions rather than copying fields. This plan takes an incremental approach — adding description fields to existing UI structs and populating them from parsed definitions, with the `LibraryManifest` extension as a foundation for the full refactor. The complete shared-ownership migration is a larger effort that can follow as a separate task.

**Tech Stack:** Rust, iced 0.14.0, flowcore model types, flowrclib parser, serde TOML/JSON

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `flowcore/src/model/flow_definition.rs` | Modify | Add `description` field |
| `flowcore/src/model/lib_manifest.rs` | Modify | Add `definitions` map for parsed `Process` objects |
| `flowedit/src/library_panel.rs` | Modify | Add `description` to `FunctionEntry`, parse TOML during scan, add tooltips |
| `flowedit/src/canvas_view.rs` | Modify | Add `description` to `NodeLayout`, two-zone hit testing |
| `flowedit/src/main.rs` | Modify | Wire description through viewer, messages, tooltip rendering |

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
Expected: PASS — existing tests should still work since `description` has `#[serde(default)]`

- [ ] **Step 6: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: No warnings or errors

- [ ] **Step 7: Commit**

```bash
git add flowcore/src/model/flow_definition.rs
git commit -m "Add optional description field to FlowDefinition (#2574)"
```

---

### Task 2: Extend `LibraryManifest` with parsed definitions

**Files:**
- Modify: `flowcore/src/model/lib_manifest.rs:51-67` (struct fields)

- [ ] **Step 1: Write test for `LibraryManifest` with definitions field**

Add this test to the existing `mod test` block in `flowcore/src/model/lib_manifest.rs`:

```rust
#[test]
fn manifest_with_definitions() {
    use crate::model::process::Process;
    use crate::model::function_definition::FunctionDefinition;

    let mut manifest = LibraryManifest::new(
        Url::parse("lib://testlib").expect("Could not parse lib url"),
        test_meta_data(),
    );

    let func = FunctionDefinition {
        name: "add".into(),
        description: "Add two numbers".into(),
        source: "add.rs".into(),
        ..Default::default()
    };

    let key = Url::parse("lib://testlib/math/add").expect("Could not parse URL");
    manifest.definitions.insert(key.clone(), Process::FunctionProcess(func));

    assert_eq!(manifest.definitions.len(), 1);
    match manifest.definitions.get(&key) {
        Some(Process::FunctionProcess(f)) => {
            assert_eq!(f.description, "Add two numbers");
        }
        _ => panic!("Expected FunctionProcess"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p flowcore manifest_with_definitions`
Expected: FAIL — no field `definitions` on `LibraryManifest`

- [ ] **Step 3: Add `definitions` field to `LibraryManifest`**

In `flowcore/src/model/lib_manifest.rs`, add import at top:

```rust
use crate::model::process::Process;
```

Add field to the struct (after `source_urls`, around line 66):

```rust
    /// Parsed definitions for each function/flow in this library.
    /// Keyed by the same `lib://` URL used in `locators`.
    /// Not serialized — populated at load time by parsing source TOMLs.
    #[serde(skip)]
    pub definitions: BTreeMap<Url, Process>,
```

Add to the `new()` constructor (after `source_urls` init, around line 78):

```rust
            definitions: BTreeMap::new(),
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p flowcore manifest_with_definitions`
Expected: PASS

- [ ] **Step 5: Run full flowcore tests**

Run: `cargo test -p flowcore`
Expected: PASS — `definitions` is `#[serde(skip)]` so serialization tests unchanged

- [ ] **Step 6: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: No warnings or errors

- [ ] **Step 7: Commit**

```bash
git add flowcore/src/model/lib_manifest.rs
git commit -m "Add definitions map to LibraryManifest for parsed Process objects (#2574)"
```

---

### Task 3: Add `description` to `FunctionEntry` and parse from TOML during library scan

**Files:**
- Modify: `flowedit/src/library_panel.rs:37-42` (FunctionEntry struct)
- Modify: `flowedit/src/library_panel.rs:328-372` (scan_functions)
- Modify: `flowedit/src/library_panel.rs:374-457` (scan_context_functions)

- [ ] **Step 1: Write test for `FunctionEntry` with description**

Add this test to the `mod test` block in `flowedit/src/library_panel.rs`:

```rust
#[test]
fn function_entry_has_description() {
    let entry = FunctionEntry {
        name: "add".into(),
        source: "lib://flowstdlib/math/add".into(),
        description: "Add two numbers".into(),
    };
    assert_eq!(entry.description, "Add two numbers");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p flowedit function_entry_has_description`
Expected: FAIL — no field `description` on `FunctionEntry`

- [ ] **Step 3: Add `description` field to `FunctionEntry`**

In `flowedit/src/library_panel.rs`, modify the `FunctionEntry` struct (line 37):

```rust
pub(crate) struct FunctionEntry {
    /// Display name of the function (e.g., "add")
    pub name: String,
    /// Source URL for this function (e.g., `lib://flowstdlib/math/add`)
    pub source: String,
    /// Description text from the function/flow definition
    pub description: String,
}
```

- [ ] **Step 4: Fix all existing `FunctionEntry` construction sites**

Update `scan_functions()` (around line 349 and 362) to add `description: String::new()` to each `FunctionEntry { ... }`. Update `scan_context_functions()` (around line 423) similarly. Update all test `FunctionEntry` constructions if any.

Verify with: `cargo build -p flowedit`
Expected: Compiles without errors

- [ ] **Step 5: Parse TOML to extract description in `scan_functions()`**

Replace the body of `scan_functions()` to parse the TOML file using flowcore's `Process` deserializer and extract the description:

```rust
fn scan_functions(cat_dir: &std::path::Path, lib_name: &str, cat_name: &str) -> Vec<FunctionEntry> {
    use flowcore::deserializers::deserializer::get;
    use flowcore::model::process::Process;
    use url::Url;

    let mut functions = Vec::new();

    let Ok(entries) = std::fs::read_dir(cat_dir) else {
        return functions;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let entry_name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            // Check if subdirectory contains a .toml file
            let toml_path = find_toml_in_dir(&path);
            if let Some(toml_file) = toml_path {
                let source = format!("lib://{lib_name}/{cat_name}/{entry_name}");
                let description = read_description_from_toml(&toml_file);
                functions.push(FunctionEntry {
                    name: entry_name,
                    source,
                    description,
                });
            }
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            let func_name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if !func_name.is_empty() {
                let source = format!("lib://{lib_name}/{cat_name}/{func_name}");
                let description = read_description_from_toml(&path);
                functions.push(FunctionEntry {
                    name: func_name,
                    source,
                    description,
                });
            }
        }
    }

    functions.sort_by(|a, b| a.name.cmp(&b.name));
    functions
}

fn find_toml_in_dir(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
        let p = e.path();
        if p.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            Some(p)
        } else {
            None
        }
    })
}

fn read_description_from_toml(toml_path: &std::path::Path) -> String {
    use flowcore::deserializers::deserializer::get;
    use flowcore::model::process::Process;
    use url::Url;

    let url = match Url::from_file_path(toml_path) {
        Ok(u) => u,
        Err(()) => return String::new(),
    };

    let content = match std::fs::read_to_string(toml_path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let deserializer = match get::<Process>(&url) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };

    match deserializer.deserialize(&content, Some(&url)) {
        Ok(Process::FunctionProcess(func)) => func.description,
        Ok(Process::FlowProcess(flow)) => flow.description,
        Err(_) => String::new(),
    }
}
```

- [ ] **Step 6: Update `scan_context_functions()` to also extract descriptions**

Apply the same pattern — find the TOML file and call `read_description_from_toml()`:

```rust
fn scan_context_functions() -> LibraryEntry {
    let mut categories = Vec::new();

    let runner_base = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".flow").join("runner"))
        .unwrap_or_default();

    if !runner_base.is_dir() {
        return LibraryEntry {
            name: "Context".to_string(),
            categories,
            expanded: true,
        };
    }

    if let Ok(runners) = std::fs::read_dir(&runner_base) {
        for runner_entry in runners.flatten() {
            let runner_path = runner_entry.path();
            if !runner_path.is_dir() {
                continue;
            }

            if let Ok(cats) = std::fs::read_dir(&runner_path) {
                for cat_entry in cats.flatten() {
                    let cat_path = cat_entry.path();
                    if !cat_path.is_dir() {
                        continue;
                    }

                    let cat_name = cat_entry.file_name().to_string_lossy().to_string();
                    let mut functions = Vec::new();

                    if let Ok(funcs) = std::fs::read_dir(&cat_path) {
                        for func_entry in funcs.flatten() {
                            let func_path = func_entry.path();
                            if func_path.extension().and_then(|e| e.to_str()) == Some("toml") {
                                let func_name = func_path
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                if !func_name.is_empty() {
                                    let source = format!("context://{cat_name}/{func_name}");
                                    let description = read_description_from_toml(&func_path);
                                    functions.push(FunctionEntry {
                                        name: func_name,
                                        source,
                                        description,
                                    });
                                }
                            }
                        }
                    }

                    if !functions.is_empty() {
                        functions.sort_by(|a, b| a.name.cmp(&b.name));
                        if !categories
                            .iter()
                            .any(|c: &CategoryEntry| c.name == cat_name)
                        {
                            categories.push(CategoryEntry {
                                name: cat_name,
                                functions,
                                expanded: false,
                            });
                        }
                    }
                }
            }
        }
    }

    categories.sort_by(|a, b| a.name.cmp(&b.name));
    LibraryEntry {
        name: "Context".to_string(),
        categories,
        expanded: true,
    }
}
```

- [ ] **Step 7: Run test to verify it passes**

Run: `cargo test -p flowedit function_entry_has_description`
Expected: PASS

- [ ] **Step 8: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: No warnings or errors

- [ ] **Step 9: Commit**

```bash
git add flowedit/src/library_panel.rs
git commit -m "Parse function descriptions from TOML during library scan (#2574)"
```

---

### Task 4: Add tooltips to library panel

**Files:**
- Modify: `flowedit/src/library_panel.rs:8-9` (imports)
- Modify: `flowedit/src/library_panel.rs:209-239` (view function, function rendering)

- [ ] **Step 1: Write test for tooltip rendering with description**

Add to the `mod test` block in `flowedit/src/library_panel.rs`:

```rust
#[test]
fn library_tree_with_descriptions_renders() {
    let tree = LibraryTree {
        libraries: vec![LibraryEntry {
            name: "testlib".into(),
            categories: vec![CategoryEntry {
                name: "math".into(),
                functions: vec![FunctionEntry {
                    name: "add".into(),
                    source: "lib://testlib/math/add".into(),
                    description: "Add two numbers".into(),
                }],
                expanded: true,
            }],
            expanded: true,
        }],
    };
    let _element: Element<'_, LibraryMessage> = tree.view();
}

#[test]
fn library_tree_empty_description_renders() {
    let tree = LibraryTree {
        libraries: vec![LibraryEntry {
            name: "testlib".into(),
            categories: vec![CategoryEntry {
                name: "math".into(),
                functions: vec![FunctionEntry {
                    name: "add".into(),
                    source: "lib://testlib/math/add".into(),
                    description: String::new(),
                }],
                expanded: true,
            }],
            expanded: true,
        }],
    };
    let _element: Element<'_, LibraryMessage> = tree.view();
}
```

- [ ] **Step 2: Run tests to verify they pass (rendering tests, no tooltip logic yet)**

Run: `cargo test -p flowedit library_tree_with_descriptions_renders library_tree_empty_description_renders`
Expected: PASS (just verifies rendering doesn't panic)

- [ ] **Step 3: Add iced `Tooltip` import and wrap function buttons**

In `flowedit/src/library_panel.rs`, update the import line (line 8):

```rust
use iced::widget::{button, container, scrollable, text, tooltip, Column, Row};
```

Then modify the function rendering loop inside `view()` (the block starting around line 209 `if cat.expanded {`). Replace the function button + row construction with tooltip wrapping:

```rust
                    if cat.expanded {
                        for func in &cat.functions {
                            let view_btn = button(text("\u{270E}").size(10))
                                .on_press(LibraryMessage::ViewFunction(
                                    func.source.clone(),
                                    func.name.clone(),
                                ))
                                .style(button::text)
                                .padding([1, 3]);

                            let func_btn = button(text(&func.name).size(11))
                                .on_press(LibraryMessage::AddFunction(
                                    func.source.clone(),
                                    func.name.clone(),
                                ))
                                .style(button::text)
                                .padding([2, 4]);

                            let row = Row::new()
                                .spacing(2)
                                .align_y(iced::Alignment::Center)
                                .push(view_btn)
                                .push(func_btn);

                            let entry_widget: Element<'_, LibraryMessage> =
                                if func.description.is_empty() {
                                    row.into()
                                } else {
                                    tooltip(
                                        row,
                                        text(&func.description).size(11),
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

                            content = content.push(container(entry_widget).padding(iced::Padding {
                                top: 0.0,
                                right: 0.0,
                                bottom: 0.0,
                                left: 24.0,
                            }));
                        }
                    }
```

- [ ] **Step 4: Verify build and tests pass**

Run: `cargo build -p flowedit && cargo test -p flowedit`
Expected: Compiles and all tests pass

- [ ] **Step 5: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: No warnings or errors

- [ ] **Step 6: Commit**

```bash
git add flowedit/src/library_panel.rs
git commit -m "Add hover tooltips showing function descriptions in library panel (#2574)"
```

---

### Task 5: Add `description` to `NodeLayout` and populate from parsed definitions

**Files:**
- Modify: `flowedit/src/canvas_view.rs:248-269` (NodeLayout struct)
- Modify: `flowedit/src/main.rs` (add_library_function, load_flow, resolve_single_function_ports)

- [ ] **Step 1: Write test for `NodeLayout` with `description` field**

Add to the `mod test` block in `flowedit/src/canvas_view.rs`:

```rust
#[test]
fn node_layout_has_description() {
    let node = NodeLayout {
        alias: "test".into(),
        source: "lib://test".into(),
        x: 0.0,
        y: 0.0,
        width: 180.0,
        height: 120.0,
        inputs: vec![],
        outputs: vec![],
        initializers: HashMap::new(),
        description: "A test function".into(),
    };
    assert_eq!(node.description, "A test function");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p flowedit node_layout_has_description`
Expected: FAIL — no field `description` on `NodeLayout`

- [ ] **Step 3: Add `description` field to `NodeLayout`**

In `flowedit/src/canvas_view.rs`, add to the `NodeLayout` struct (after `initializers`, around line 268):

```rust
    /// Description text from the function/flow definition
    pub description: String,
```

- [ ] **Step 4: Fix all `NodeLayout` construction sites**

There are many places that construct `NodeLayout`. Add `description: String::new()` to each. These are found in:
- `flowedit/src/main.rs` in `add_library_function()` (around line 2816)
- `flowedit/src/main.rs` in `load_flow()` (where nodes are built from process references)
- `flowedit/src/main.rs` in various `WindowState` creation points
- `flowedit/src/canvas_view.rs` in tests

Search for `NodeLayout {` to find all sites. Add `description: String::new()` to each one initially.

Run: `cargo build -p flowedit`
Expected: Compiles

- [ ] **Step 5: Populate description from parsed definitions**

Modify `resolve_single_function_ports()` in `main.rs` (around line 3178) to also return the description. Change its signature to return a tuple of three items:

```rust
fn resolve_single_function_ports(
    source: &str,
    base_url: Option<&Url>,
) -> (Vec<PortInfo>, Vec<PortInfo>, String) {
```

Update the return values in the match (around line 3224):

```rust
    match deserializer.deserialize(&content, Some(&resolved_url)) {
        Ok(Process::FunctionProcess(func)) => {
            let (inputs, outputs) = extract_ports(&func.inputs, &func.outputs);
            (inputs, outputs, func.description)
        }
        Ok(Process::FlowProcess(flow)) => {
            let (inputs, outputs) = extract_ports(&flow.inputs, &flow.outputs);
            (inputs, outputs, flow.description)
        }
        Err(_) => (Vec::new(), Vec::new(), String::new()),
    }
```

Update the early return statements to return three items: `(Vec::new(), Vec::new(), String::new())`

Then update `add_library_function()` to use the description:

```rust
    let (inputs, outputs, description) = resolve_single_function_ports(source, None);

    let node = NodeLayout {
        alias: alias.clone(),
        source: source.to_string(),
        x,
        y,
        width: 180.0,
        height: 120.0,
        inputs,
        outputs,
        initializers: HashMap::new(),
        description,
    };
```

And update `load_flow()` similarly — where `resolve_single_function_ports` is called to build nodes from process references, capture and use the description.

- [ ] **Step 6: Run tests**

Run: `cargo test -p flowedit`
Expected: PASS

- [ ] **Step 7: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: No warnings or errors

- [ ] **Step 8: Commit**

```bash
git add flowedit/src/canvas_view.rs flowedit/src/main.rs
git commit -m "Add description field to NodeLayout and populate from definitions (#2574)"
```

---

### Task 6: Two-zone canvas tooltips

**Files:**
- Modify: `flowedit/src/canvas_view.rs:1456-1474` (hover detection)
- Modify: `flowedit/src/canvas_view.rs:2269-2285` (source text positioning constants)

- [ ] **Step 1: Write test for source text hit testing**

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
        description: "Adds numbers".into(),
    };
    // Source text is centered at (node.x + width/2, node.y + 34.0)
    // with approximate height SOURCE_FONT_SIZE and width proportional to text
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

Add this function near `hit_test_node` in `flowedit/src/canvas_view.rs`:

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

- [ ] **Step 5: Modify hover detection to use two zones**

In `flowedit/src/canvas_view.rs`, replace the hover tracking block (around line 1456-1474):

```rust
                    // Track hover for two-zone node tooltip
                    let new_hover = hit_test_node(self.nodes, world_pos);
                    if new_hover != state.hover_node {
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
                                } else if !n.description.is_empty() {
                                    Some((n.description.clone(), bottom_center.x, bottom_center.y))
                                } else {
                                    None
                                }
                            });
                        return Some(canvas::Action::publish(CanvasMessage::HoverChanged(
                            tooltip_data,
                        )));
                    }
```

Also update the tooltip when cursor moves within the same node (the hover_node hasn't changed but the zone may have). Add a check after the `if new_hover != state.hover_node` block:

```rust
                    // Even if hover node didn't change, the zone may have
                    if let Some(idx) = state.hover_node {
                        if let Some(n) = self.nodes.get(idx) {
                            let bottom_center = transform_point(
                                Point::new(n.x + n.width / 2.0, n.y + n.height),
                                zoom,
                                offset,
                            );
                            let tooltip_data = if is_in_source_text_zone(n, world_pos) {
                                Some((n.source.clone(), bottom_center.x, bottom_center.y))
                            } else if !n.description.is_empty() {
                                Some((n.description.clone(), bottom_center.x, bottom_center.y))
                            } else {
                                None
                            };
                            return Some(canvas::Action::publish(CanvasMessage::HoverChanged(
                                tooltip_data,
                            )));
                        }
                    }
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p flowedit`
Expected: PASS

- [ ] **Step 7: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: No warnings or errors

- [ ] **Step 8: Commit**

```bash
git add flowedit/src/canvas_view.rs
git commit -m "Add two-zone canvas tooltips: source text vs description (#2574)"
```

---

### Task 7: Editable description field in function viewer

**Files:**
- Modify: `flowedit/src/main.rs:86-87` (Message enum)
- Modify: `flowedit/src/main.rs:172-182` (FunctionViewer struct)
- Modify: `flowedit/src/main.rs:978-985` (update handler)
- Modify: `flowedit/src/main.rs:1889-1896` (view rendering)

- [ ] **Step 1: Add `description` field to `FunctionViewer` struct**

In `flowedit/src/main.rs`, add to the `FunctionViewer` struct (after `name`, around line 174):

```rust
struct FunctionViewer {
    name: String,
    description: String,
    source_file: String,
    inputs: Vec<PortInfo>,
    outputs: Vec<PortInfo>,
    rs_content: String,
    docs_content: Option<String>,
    active_tab: usize,
    toml_path: PathBuf,
}
```

- [ ] **Step 2: Add `FunctionDescriptionChanged` message variant**

In the `Message` enum (after `FunctionNameChanged`, around line 87):

```rust
    /// Function description edited
    FunctionDescriptionChanged(window::Id, String),
```

- [ ] **Step 3: Add update handler for description changes**

In the `update()` method, after the `FunctionNameChanged` handler (around line 985):

```rust
            Message::FunctionDescriptionChanged(win_id, new_desc) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref mut viewer) = win.kind {
                        viewer.description = new_desc;
                    }
                    win.unsaved_edits += 1;
                }
            }
```

- [ ] **Step 4: Add description text_input to function viewer rendering**

In the view rendering for function viewer (after the `name_input` container, around line 1896), add:

```rust
                let desc_input = container(
                    text_input("Description", &viewer.description)
                        .on_input(move |s| Message::FunctionDescriptionChanged(window_id, s))
                        .size(13)
                        .padding(6)
                        .width(480),
                )
                .center_x(Fill);
```

Then add `desc_input` to the `func_box` Column (after `.push(name_input)`, around line 1935):

```rust
                        .push(name_input)
                        .push(desc_input)
```

- [ ] **Step 5: Fix all `FunctionViewer` construction sites**

Search for `FunctionViewer {` in `main.rs` and add `description: String::new()` (or the actual description from the parsed definition) to each construction site.

Where the viewer is opened from a parsed function definition, capture the description:

```rust
description: func.description.clone(),
```

For flow definitions:

```rust
description: flow.description.clone(),
```

Run: `cargo build -p flowedit`
Expected: Compiles

- [ ] **Step 6: Wire description back to TOML on save**

Find where the function viewer saves back to TOML (search for `toml_path` usage in save/write contexts). Ensure the description field is included when the function definition is serialized back.

This should work automatically since `FunctionDefinition` and `FlowDefinition` already serialize the `description` field. Just ensure the viewer's description is written back to the definition before serialization.

- [ ] **Step 7: Run tests**

Run: `cargo test -p flowedit`
Expected: PASS

- [ ] **Step 8: Run full `make test`**

Run: `make test`
Expected: PASS

- [ ] **Step 9: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: No warnings or errors

- [ ] **Step 10: Commit**

```bash
git add flowedit/src/main.rs
git commit -m "Add editable description field to function viewer window (#2574)"
```

---

### Task 8: Manual testing and final verification

**Files:** None (testing only)

- [ ] **Step 1: Run full `make test`**

Run: `make test`
Expected: All tests pass

- [ ] **Step 2: Run `make clippy` and `cargo fmt`**

Run: `make clippy && cargo fmt`
Expected: Clean

- [ ] **Step 3: Launch flowedit and test palette tooltips**

Run: `cargo run -p flowedit -- examples/fibonacci/fibonacci.toml`

Verify:
- Hover over functions in the library panel — tooltip shows description text
- Functions with no description show no tooltip
- Context functions show descriptions (read-only)

- [ ] **Step 4: Test canvas two-zone tooltips**

In the loaded flow:
- Hover over a node's source text — tooltip shows the full source path
- Hover over rest of node — tooltip shows the description
- Nodes with empty descriptions show no tooltip on the body area

- [ ] **Step 5: Test description editing in function viewer**

- Click the pencil icon on a function node to open the viewer
- Verify description text_input appears below the name
- Edit the description
- Verify the canvas tooltip updates with the new description

- [ ] **Step 6: Test saving and reloading**

- Edit a description on a provided implementation node
- Save the flow
- Close and reopen the flow
- Verify the description persists

- [ ] **Step 7: Final commit if any adjustments were needed**

```bash
git add -A
git commit -m "Polish description tooltips after manual testing (#2574)"
```
