# flowedit: Encapsulate View Rendering and Message Sub-types (#2593/#2597)

## Overview

Sub-project 2 of #2593, combined with #2597. Move view rendering code
from main.rs into the modules that own the state, and create message
sub-enums to group related messages.

## Goals

1. Move canvas-related view code (zoom controls, tooltip overlay,
   initializer editor overlay, context menu overlay) from main.rs
   `view()` into `canvas_view.rs` as a `view_canvas_area()` method
2. Move metadata editor panel rendering into a helper
3. Move status bar/toolbar rendering into a helper
4. Create message sub-enums: `FlowEditMessage` and `FunctionEditMessage`
   to group related variants (initializer messages remain as top-level
   `Message` variants since they are already routed with window IDs)
5. Move their handlers into the appropriate modules

## Canvas View Encapsulation

`canvas_view.rs` gets a new public function:

```text
pub(crate) fn view_canvas_area<'a>(
    win: &'a WindowState,
    window_id: window::Id,
) -> Element<'a, Message>
```

This returns the complete canvas area including:
- The interactive canvas widget
- Zoom control buttons (+, -, Fit) overlaid in the corner
- Tooltip overlay (if hovering)
- Initializer editor dialog (if editing)
- Context menu overlay (if right-clicked)

main.rs `view()` just calls this function instead of building all the
overlays inline.

## Message Sub-enums

Group flat `Message` variants into sub-enums routed through wrapper
variants:

```text
Message::FlowEdit(window::Id, FlowEditMessage)
Message::FunctionEdit(window::Id, FunctionEditMessage)
```

Where:
- `FlowEditMessage` contains: NameChanged, VersionChanged,
  DescriptionChanged, AuthorsChanged, ToggleMetadataEditor,
  AddInput, AddOutput, DeleteInput, DeleteOutput,
  InputNameChanged, InputTypeChanged, OutputNameChanged,
  OutputTypeChanged
- `FunctionEditMessage` contains: TabSelected, NameChanged,
  DescriptionChanged, BrowseSource, AddInput, AddOutput,
  DeleteInput, DeleteOutput, InputNameChanged, InputTypeChanged,
  OutputNameChanged, OutputTypeChanged, Save

The existing `InitializerMessage` handlers are already extracted to
`initializer.rs` — just need to group the message variants.

## Testing

Pure refactor — all 180 existing tests must pass. No new tests needed.

## Key Files

| File | Change |
|------|--------|
| `flowedit/src/canvas_view.rs` | Add `view_canvas_area()` |
| `flowedit/src/main.rs` | Replace inline view code with calls; add sub-enums |
