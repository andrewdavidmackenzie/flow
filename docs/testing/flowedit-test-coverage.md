# flowedit Test Coverage Tracking

Based on the user manual (`book/editing/flowedit.md`), this document tracks
which features and interactions are covered by automated tests.

Legend: `[x]` = tested, `[ ]` = not tested

---

## Launching
- [ ] Open an existing flow file from CLI
- [ ] Start with empty canvas (no file argument)
- [ ] `-L` / `--libdir` adds library search paths

## Nodes
- [x] Node color by source type (fill_color_by_source)
- [x] Node label derives short name (derive_short_name_*)
- [x] Port positions on node edges (node_layout_port_positions)
- [x] Port name resolution with array subroutes (base_port_name_*, find_node_output_inline_with_subroute)
- [ ] Input initializer yellow port color
- [ ] Initializer value display text

### Resizing Nodes
- [x] Resize message updates dimensions (update_canvas_resize_node)
- [ ] Resize handles visible on selected node
- [ ] Resize handle cursor changes

## Connections
- [x] Edge layout from connections (build_edge_layouts_single)
- [x] Edge references node check (edge_references_node)
- [x] Connection hit testing — segment distance (distance_to_segment_*)
- [x] Cubic bezier endpoints (cubic_bezier_endpoints)
- [x] Quadratic bezier endpoints (quadratic_bezier_endpoints)
- [x] Port type compatibility — same type (check_type_compat_same_type)
- [x] Port type compatibility — different type (check_type_compat_different_type)
- [x] Port type compatibility — untyped allows any (check_type_compat_untyped_allows_any)
- [ ] Connection arrow head drawing
- [ ] Connection name label display
- [ ] Loopback connection routing
- [ ] Flow I/O connection drawing
- [x] Flow I/O port positions (compute_flow_io_positions_*)

## Interactions

### Selecting Nodes
- [x] Click on node selects it — update handler (update_canvas_select_node)
- [x] Click on node selects it — UI simulator (canvas_click_selects_node)
- [x] Click empty canvas deselects — update handler (update_canvas_deselect)
- [x] Click empty canvas deselects — UI simulator (canvas_click_empty_deselects)
- [x] Hit test node inside bounds (hit_test_node_inside)
- [x] Hit test node outside bounds (hit_test_node_outside, hit_test_node_miss)

### Moving Nodes
- [x] Move node updates position (update_canvas_move_node)
- [x] Move completed records history (update_canvas_move_completed_records_history)
- [ ] Drag cursor changes to grabbing

### Deleting Nodes
- [x] Delete node removes from list (update_canvas_delete_node)
- [x] Delete increments unsaved edits
- [ ] Delete removes connected edges

### Creating Connections
- [x] Create connection adds edge (update_canvas_create_connection)
- [x] Create connection increments unsaved edits
- [ ] Drag preview bezier curve
- [ ] Compatible port highlighting during drag
- [ ] Crosshair cursor over ports

### Selecting Connections
- [x] Select connection updates state (update_canvas_select_connection)
- [ ] Selected connection yellow highlight
- [ ] Flow I/O connection selection highlight

### Deleting Connections
- [x] Delete connection removes edge (update_canvas_delete_connection)

## Layout
- [x] Topological auto-layout (implicit via build_node_layouts)
- [x] Split route parsing (split_route_*)
- [x] Source truncation (truncate_source_*)
- [ ] Saved layout positions preserved on load
- [ ] Grid fallback layout

## Zoom and Scroll
- [x] Zoom in button (click_zoom_in, update_zoom_in)
- [x] Zoom out button (click_zoom_out, update_zoom_out)
- [x] Zoom in/out roundtrip (zoom_in_out_roundtrip)
- [x] Fit button enables auto-fit (click_fit_enables_auto_fit, update_toggle_auto_fit)
- [x] Transform point with zoom (transform_point_with_zoom)
- [x] Transform point with offset (transform_point_with_offset)
- [x] Screen-to-world roundtrip (transform_and_inverse, screen_to_world_roundtrip)
- [ ] Scroll wheel panning
- [ ] Cmd+scroll wheel zooming
- [ ] Middle-mouse button panning

## Undo / Redo
- [x] Record and undo (record_and_undo, record_and_undo_edit)
- [x] Redo after undo (redo_after_undo)
- [x] New action clears redo stack (new_action_clears_redo)
- [x] Undo empty returns None (undo_empty)
- [x] Redo empty returns None (redo_empty)
- [x] Update undo/redo cycle (update_undo_redo_cycle)
- [x] Delete node roundtrip (delete_node_roundtrip)
- [x] Create connection roundtrip (create_connection_roundtrip)

## File Operations
- [x] Save flow TOML roundtrip (save_and_load_flow_roundtrip)
- [x] Save with metadata (save_flow_with_metadata)
- [x] Save with initializers (save_flow_with_initializers)
- [x] Save with connections (save_flow_with_connections)
- [x] Perform save updates state (perform_save_updates_state)
- [x] Load nonexistent file returns error (load_flow_nonexistent)
- [x] Load invalid TOML returns error (load_flow_invalid_toml)
- [x] Sync flow definition preserves nodes (sync_flow_definition_preserves_nodes)
- [ ] Save As dialog (requires rfd)
- [ ] Open dialog (requires rfd)
- [ ] New flow resets state

## Process Library
- [x] Library tree view renders (empty_library_tree_view)
- [x] Toggle library expansion (toggle_library_expansion)
- [x] Toggle category expansion (toggle_category_expansion)
- [x] Toggle out of bounds does not panic
- [x] Add function returns source (add_function_returns_source)
- [x] Resolve lib path includes default
- [ ] Library function view button (pencil icon)

## Flow Hierarchy
- [x] Empty hierarchy (empty_hierarchy)
- [ ] Build hierarchy from flow file
- [ ] Toggle expand/collapse
- [ ] Open sub-flow from hierarchy
- [ ] Open function from hierarchy

## Creating New Processes
- [ ] New Sub-flow button creates flow file
- [ ] New Function button opens editor
- [x] Right-click context menu show/dismiss (update_context_menu)
- [ ] Context menu "New Sub-flow" action
- [ ] Context menu "New Function" action

## Opening Sub-flows and Functions
- [x] Openable node detection — lib (is_openable_lib)
- [x] Openable node detection — context (is_openable_context)
- [x] Openable node detection — local (is_openable_local)
- [x] Open icon hit test (hit_test_open_icon_only_openable)
- [ ] Pencil icon opens sub-flow window
- [ ] Pencil icon opens function editor
- [ ] Duplicate window prevention (focuses existing)
- [x] Resolve node source with .toml extension (resolve_node_source_toml_extension)
- [x] Resolve node source not found (resolve_node_source_not_found)

### Sub-flow Windows
- [ ] Bounding box drawn around nodes
- [ ] Flow name on bounding box
- [ ] Flow I/O ports on bounding box edges
- [ ] Flow I/O bezier connections drawn

### Function Definition Editor
- [x] Save function creates .toml, .rs, function.toml (save_function_definition_creates_files)
- [x] Existing .rs not overwritten (save_function_no_overwrite_existing_rs)
- [ ] Function name editing
- [ ] Port add/delete
- [ ] Source file link opens source view
- [ ] Docs tab

## Compiling
- [x] Build button with saved flow (click_build_with_saved_flow)
- [ ] Build with unsaved flow prompts Save As
- [ ] Compile error displayed in status

## Metadata Editor
- [x] Info button toggles panel — update (update_toggle_metadata)
- [x] Info button toggles panel — UI (click_info_toggles_metadata)
- [x] Panel shows Name/Version fields (metadata_panel_shows_fields)
- [x] Flow name change (update_flow_name_changed)
- [x] Flow version change (update_flow_version_changed)
- [x] Flow description change (update_flow_description_changed)
- [x] Flow authors change (update_flow_authors_changed)

## Library Paths
- [x] Libs button toggles panel — update (update_toggle_lib_paths)
- [x] Libs button toggles panel — UI (click_libs_toggles_panel)
- [ ] Add library path via folder picker
- [ ] Remove library path
- [ ] Library panel rescans on path change

## Window Position Persistence
- [x] Editor prefs path format (editor_prefs_path_format)
- [x] Editor prefs save/load roundtrip (editor_prefs_roundtrip)
- [x] Missing prefs file returns None (editor_prefs_no_file)

## Window Management
- [x] Window focus tracking (update_window_focused)
- [x] Window resize tracking (update_window_resized)
- [x] Window move tracking (update_window_moved)
- [ ] Cmd+W closes focused window
- [ ] Cmd+Q prompts for unsaved changes
- [ ] Closing root window exits app
- [ ] Closing child window removes state
- [ ] Window re-open after close via decoration

## Flow I/O Editing
- [x] Add flow input (update_flow_add_input)
- [x] Add flow output (update_flow_add_output)
- [x] Delete flow input (update_flow_delete_input)
- [x] Change flow input name (update_flow_input_name_changed)
- [ ] Change flow input type
- [ ] Change flow output name/type

## Initializer Editing
- [x] Initializer type change (update_initializer_type_changed)
- [x] Initializer cancel (update_initializer_cancel)
- [x] Initializer TOML serialization — once (initializer_to_toml_once)
- [x] Initializer TOML serialization — always (initializer_to_toml_always)
- [ ] Right-click on input port opens editor
- [ ] Apply initializer saves value

## Value Serialization
- [x] String to TOML (value_to_toml_string)
- [x] Number to TOML (value_to_toml_number)
- [x] Bool to TOML (value_to_toml_bool)
- [x] Array to TOML (value_to_toml_array)
- [x] Format value display — all types (format_value_*)

## Alias Generation
- [x] No conflict (unique_alias_no_conflict)
- [x] Single conflict (unique_alias_with_conflict)
- [x] Multiple conflicts (unique_alias_multiple_conflicts)
- [x] Next node position — empty (next_position_empty)
- [x] Next node position — after nodes (next_position_after_nodes)

## Miscellaneous
- [x] Format endpoint with port (format_endpoint_*)
- [x] View renders without crash (find_status_text)
