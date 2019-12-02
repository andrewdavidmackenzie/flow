use gtk::TextBuffer;

use flowrlib::manifest::Manifest;

/// `UIContext` Holds items from the UI that are needed during runnings of a flow. But that will
/// only be written to by the UI, on the UI Thread
#[derive(Clone)]
pub struct UIContext {
    pub flow: TextBuffer,
    pub manifest_buffer: TextBuffer,
    pub manifest: Option<Manifest>
    // TODO Log level elector
    // TODO log output with a new logger
    // TODO stdin
    // TODO flow lib path
}

impl UIContext {
    pub fn new() -> Self {
        UIContext {
            flow: TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE),
            manifest_buffer: TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE),
            manifest: None
        }
    }
}