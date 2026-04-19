# Multi-Window Daemon Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor flowedit from `iced::application` (single window) to `iced::daemon` (multi-window) so sub-flows can be opened in new windows within the same process, sharing state and lifecycle.

**Architecture:** Extract per-window state into a `WindowState` struct, keyed by `iced::window::Id` in a `HashMap`. The top-level `FlowEdit` becomes the app-wide state holding the window map, library tree, and shared config. The daemon's `view(state, window_id)` and `title(state, window_id)` dispatch to the correct window. Window lifecycle (open/close/exit) is handled via `iced::window::open/close/close_requests` and `iced::exit`.

**Tech Stack:** Rust, iced 0.14.0 (`daemon` API, `window::open`, `window::close`, `window::close_requests`)

---

## File Map

| File | Changes |
|------|---------|
| `flowedit/src/main.rs` | Major: extract `WindowState`, switch `main()` to `daemon()`, add `window::Id` routing to `view`/`title`/`update`, add window lifecycle messages, rewrite `open_node` to use `window::open` |
| `flowedit/src/canvas_view.rs` | None (already receives data by reference, no state changes needed) |
| `flowedit/src/history.rs` | None (already per-window, just needs to live inside `WindowState`) |
| `flowedit/src/library_panel.rs` | None (shared across windows, stays in top-level state) |

All changes are in `flowedit/src/main.rs`. The refactor is large but confined to one file.

---

### Task 1: Extract WindowState struct

Extract per-window fields from `FlowEdit` into a new `WindowState` struct. Keep `FlowEdit` as app-wide state with a `HashMap<window::Id, WindowState>`.

**Files:**
- Modify: `flowedit/src/main.rs:94-129` (struct FlowEdit)

- [ ] **Step 1: Define WindowState and refactor FlowEdit**

Add `WindowState` struct above `FlowEdit`. Move per-window fields into it. Add `windows` map and `root_window` to `FlowEdit`.

```rust
use iced::window;

/// Per-window editor state.
struct WindowState {
    flow_name: String,
    nodes: Vec<NodeLayout>,
    edges: Vec<EdgeLayout>,
    canvas_state: FlowCanvasState,
    status: String,
    selected_node: Option<usize>,
    selected_connection: Option<usize>,
    history: EditHistory,
    auto_fit_pending: bool,
    auto_fit_enabled: bool,
    unsaved_edits: i32,
    compiled_manifest: Option<PathBuf>,
    file_path: Option<PathBuf>,
    flow_definition: FlowDefinition,
    tooltip: Option<(String, f32, f32)>,
    initializer_editor: Option<InitializerEditor>,
    is_root: bool,
}

struct FlowEdit {
    windows: HashMap<window::Id, WindowState>,
    root_window: Option<window::Id>,
    library_tree: LibraryTree,
}
```

- [ ] **Step 2: Add WindowState constructor**

```rust
impl WindowState {
    fn new(
        flow_name: String,
        nodes: Vec<NodeLayout>,
        edges: Vec<EdgeLayout>,
        file_path: Option<PathBuf>,
        flow_definition: FlowDefinition,
        is_root: bool,
    ) -> Self {
        let has_nodes = !nodes.is_empty();
        Self {
            flow_name,
            nodes,
            edges,
            canvas_state: FlowCanvasState::default(),
            status: String::from("Ready"),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: has_nodes,
            auto_fit_enabled: true,
            unsaved_edits: 0,
            compiled_manifest: None,
            file_path,
            flow_definition,
            tooltip: None,
            initializer_editor: None,
            is_root,
        }
    }
}
```

- [ ] **Step 3: Build — expect many errors**

Run: `cargo build -p flowedit 2>&1 | head -30`
Expected: Many errors because all the methods still reference `self.nodes`, `self.edges`, etc. directly on `FlowEdit`. This confirms the struct split compiled but methods need updating.

- [ ] **Step 4: Commit checkpoint**

```bash
git add flowedit/src/main.rs
git commit -m "WIP: extract WindowState struct from FlowEdit"
```

---

### Task 2: Switch main() to daemon and open root window

Change `main()` from `iced::application()` to `iced::daemon()`. Open the root window in `new()` via `window::open()`. Add window lifecycle messages.

**Files:**
- Modify: `flowedit/src/main.rs:44-79` (Message enum)
- Modify: `flowedit/src/main.rs:134-141` (main function)
- Modify: `flowedit/src/main.rs:145-257` (FlowEdit::new)

- [ ] **Step 1: Add window lifecycle messages**

Add to the `Message` enum:

```rust
enum Message {
    // ... existing variants ...
    /// A window was opened (the task from window::open completed)
    WindowOpened(window::Id),
    /// The user requested closing a window (clicked the X button)
    CloseRequested(window::Id),
}
```

- [ ] **Step 2: Switch main() to daemon**

```rust
fn main() -> iced::Result {
    env_logger::init();
    iced::daemon(FlowEdit::new, FlowEdit::update, FlowEdit::view)
        .title(FlowEdit::title)
        .subscription(FlowEdit::subscription)
        .antialiasing(true)
        .run()
}
```

- [ ] **Step 3: Update new() to open root window**

In `new()`, after parsing CLI args and loading the flow, open the root window via `window::open()` and store the window state:

```rust
fn new() -> (Self, Task<Message>) {
    // ... existing CLI parsing and flow loading ...

    let library_tree = LibraryTree::scan();

    let (root_id, open_task) = window::open(window::Settings {
        size: iced::Size::new(1024.0, 768.0),
        ..Default::default()
    });

    let win_state = WindowState::new(
        flow_name, nodes, edges, file_path, flow_definition, true,
    );

    let mut windows = HashMap::new();
    windows.insert(root_id, win_state);

    let app = FlowEdit {
        windows,
        root_window: Some(root_id),
        library_tree,
    };

    (app, open_task.discard())
}
```

- [ ] **Step 4: Update title() to accept window::Id**

```rust
fn title(&self, window_id: window::Id) -> String {
    let Some(win) = self.windows.get(&window_id) else {
        return String::from("flowedit");
    };
    let modified = if win.unsaved_edits > 0 { " *" } else { "" };
    let file = win
        .file_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("untitled");
    format!("flowedit - {} ({}){modified}", win.flow_name, file)
}
```

- [ ] **Step 5: Update view() to accept window::Id**

```rust
fn view(&self, window_id: window::Id) -> Element<'_, Message> {
    let Some(win) = self.windows.get(&window_id) else {
        return Text::new("Window not found").into();
    };
    // ... existing view code, but reading from `win` instead of `self` ...
    // The compile button is only shown for root windows (win.is_root)
}
```

- [ ] **Step 6: Update subscription() with close_requests**

```rust
fn subscription(&self) -> Subscription<Message> {
    Subscription::batch(vec![
        keyboard::listen().filter_map(|event| match event {
            // ... existing keyboard handling ...
        }),
        window::close_requests().map(Message::CloseRequested),
    ])
}
```

- [ ] **Step 7: Handle CloseRequested in update()**

When any window close is requested: if it's the root window, exit the entire app. If it's a child window, just close that window.

```rust
Message::CloseRequested(id) => {
    if self.root_window == Some(id) {
        // Root window closed — exit the whole app
        return iced::exit();
    }
    // Child window — just close it
    self.windows.remove(&id);
    return window::close(id);
}
```

- [ ] **Step 8: Build and fix compilation errors**

Run: `cargo build -p flowedit 2>&1`

This is the main integration step. All the `self.nodes`, `self.edges`, etc. references in methods like `update()` need to become `win.nodes`, `win.edges` where `win` is looked up from `self.windows` using the appropriate window ID.

For `update()`, most messages need a "current window" context. Since iced's `update` doesn't receive a window ID, messages that come from canvas interactions need to carry the window ID. The approach: wrap canvas messages with a window ID.

Add a new message variant:
```rust
/// A message from a specific window's canvas
WindowCanvas(window::Id, CanvasMessage),
```

And in `view()`, map canvas messages to include the window ID:
```rust
let canvas = win.canvas_state
    .view(&win.nodes, &win.edges, win.auto_fit_pending, win.auto_fit_enabled)
    .map(move |msg| Message::WindowCanvas(window_id, msg));
```

Then in `update()`, extract the window ID from `WindowCanvas` and look up the right `WindowState`.

- [ ] **Step 9: Commit**

```bash
git add flowedit/src/main.rs
git commit -m "Switch from iced::application to iced::daemon with multi-window support"
```

---

### Task 3: Migrate all update handlers to per-window dispatch

Route every message to the correct `WindowState` via the window ID. This is the bulk of the mechanical refactor.

**Files:**
- Modify: `flowedit/src/main.rs` (update method and all helper methods)

- [ ] **Step 1: Add active_window tracking**

Since some messages (keyboard shortcuts, library panel) don't carry a window ID, track the "focused" window. Use `window::events()` to detect focus changes, or default to the root window.

A simpler approach: keyboard shortcuts and library panel additions always target the root window (or most recently focused). For now, use `root_window` as the default target for non-window-specific messages.

- [ ] **Step 2: Refactor update() message routing**

The pattern for each message is:
1. Determine which window it targets (from the message or default to root)
2. Get `&mut WindowState` from `self.windows`
3. Perform the operation on that window state

Example for `Message::Save`:
```rust
Message::Save => {
    if let Some(win) = self.root_window.and_then(|id| self.windows.get_mut(&id)) {
        if let Some(path) = win.file_path.clone() {
            Self::perform_save(win, &path);
        } else {
            Self::perform_save_as(win);
        }
    }
}
```

- [ ] **Step 3: Convert instance methods to take &mut WindowState**

Methods like `perform_save`, `perform_open`, `record_edit`, `apply_undo`, `apply_redo`, `apply_initializer_edit`, `sync_flow_definition`, `add_library_function`, `perform_compile`, etc. currently take `&mut self` (FlowEdit). They need to either:
- Become associated functions taking `&mut WindowState`
- Or take a window ID parameter and look up the window

The associated function approach is cleaner:
```rust
fn perform_save(win: &mut WindowState, path: &PathBuf) { ... }
fn record_edit(win: &mut WindowState, action: EditAction) { ... }
```

- [ ] **Step 4: Build and iterate on errors**

Run: `cargo build -p flowedit 2>&1`
Fix errors one by one. This is mechanical but tedious.

- [ ] **Step 5: Run tests**

Run: `make test 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add flowedit/src/main.rs
git commit -m "Route all update handlers through per-window WindowState"
```

---

### Task 4: Implement open_node with window::open

Replace the subprocess spawn in `open_node` with `window::open()` to create a new in-process window.

**Files:**
- Modify: `flowedit/src/main.rs` (open_node method)

- [ ] **Step 1: Rewrite open_node to use window::open**

```rust
fn open_node(&mut self, window_id: window::Id, idx: usize) -> Task<Message> {
    let Some(win) = self.windows.get(&window_id) else {
        return Task::none();
    };
    let Some(node) = win.nodes.get(idx) else {
        return Task::none();
    };
    let source = node.source.clone();

    let Some(path) = Self::resolve_node_source(win, &source) else {
        if let Some(w) = self.windows.get_mut(&window_id) {
            w.status = format!("Could not resolve source: {source}");
        }
        return Task::none();
    };

    // Load the flow from the resolved path
    match load_flow(&path) {
        Ok((name, nodes, edges, flow_def)) => {
            let (new_id, open_task) = window::open(window::Settings {
                size: iced::Size::new(1024.0, 768.0),
                ..Default::default()
            });
            let child = WindowState::new(
                name, nodes, edges, Some(path.clone()), flow_def, false,
            );
            self.windows.insert(new_id, child);
            if let Some(w) = self.windows.get_mut(&window_id) {
                w.status = format!("Opened: {}", path.display());
            }
            open_task.discard()
        }
        Err(e) => {
            // Could be a function definition — show status
            if let Some(w) = self.windows.get_mut(&window_id) {
                w.status = format!("Could not open '{}': {e}", source);
            }
            Task::none()
        }
    }
}
```

- [ ] **Step 2: Update Message::WindowCanvas(OpenNode) handler to return Task**

In the `update()` match arm for `OpenNode`:
```rust
CanvasMessage::OpenNode(idx) => {
    return self.open_node(win_id, idx);
}
```

Note: `update()` already returns `Task<Message>`, so this works. Just make sure the other arms that currently don't return a task still return `Task::none()`.

- [ ] **Step 3: Child windows hide compile button**

In `view()`, when building the status bar for a window, check `win.is_root`:
```rust
if win.is_root {
    // Show compile button
} else {
    // No compile button for child windows
}
```

- [ ] **Step 4: Build and test manually**

Run: `cargo build -p flowedit && cargo run -p flowedit -- flowr/examples/mandlebrot/root.toml`

Click the ↗ icon on a sub-flow node. A new window should open within the same process showing the sub-flow's contents.

- [ ] **Step 5: Run tests**

Run: `cargo fmt -p flowedit && cargo clippy -p flowedit && make test 2>&1 | tail -5`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add flowedit/src/main.rs
git commit -m "Open sub-flows in new in-process windows via iced::daemon"
```

---

### Task 5: Window close lifecycle and exit

Ensure closing any child window removes its state. Closing the root window exits the entire application.

**Files:**
- Modify: `flowedit/src/main.rs` (subscription, update)

- [ ] **Step 1: Handle unsaved changes on close**

Before closing a window with unsaved edits, prompt the user:
```rust
Message::CloseRequested(id) => {
    if let Some(win) = self.windows.get(&id) {
        if win.unsaved_edits > 0 {
            // For now, just close without prompting (Phase 7 will add save prompt)
        }
    }
    if self.root_window == Some(id) {
        return iced::exit();
    }
    self.windows.remove(&id);
    return window::close(id);
}
```

- [ ] **Step 2: Test close behavior**

Open mandlebrot, click ↗ on a sub-flow. Close the child window — root should stay open. Close the root window — app should exit entirely.

- [ ] **Step 3: Commit**

```bash
git add flowedit/src/main.rs
git commit -m "Handle window close lifecycle: child closes independently, root exits app"
```

---

### Task 6: Final cleanup and manual testing

- [ ] **Step 1: Run full test suite**

```bash
cargo fmt -p flowedit && cargo clippy -p flowedit && make test
```

- [ ] **Step 2: Manual test with mandlebrot**

```bash
cargo run -p flowedit -- flowr/examples/mandlebrot/root.toml
```

Verify:
- Root window shows "flowedit - my-mandlebrot (root.toml)"
- Sub-flow nodes (generate_pixels, render, parse_args) show ↗ icon
- lib:// and context:// nodes do NOT show ↗ icon
- Clicking ↗ opens sub-flow in a new window within the same process
- Child window title shows the sub-flow name
- Child window has no Compile button
- Closing child window leaves root open
- Closing root window exits the application

- [ ] **Step 3: Manual test with reverse-echo**

```bash
cargo run -p flowedit -- flowr/examples/reverse-echo/root.toml
```

Verify:
- `reverse` node (provided implementation) shows ↗ icon
- Clicking ↗ shows status message "Could not open 'reverse/reverse': ..." (function, not a flow)

- [ ] **Step 4: Manual test with fibonacci**

```bash
cargo run -p flowedit -- flowr/examples/fibonacci/root.toml
```

Verify:
- `add` (lib://) and `stdout` (context://) nodes do NOT show ↗ icon
- All existing features (drag, connect, resize, undo, save, compile) still work

- [ ] **Step 5: Commit final**

```bash
git add -A
git commit -m "Phase 6 Step 1: Multi-window daemon with sub-flow opening"
```
