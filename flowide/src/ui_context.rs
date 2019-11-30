use gtk::TextBuffer;

/// `UIContext` Holds items from the UI that are needed during runnings of a flow. But that will
/// only be written to by the UI, on the UI Thread
#[derive(Clone)]
pub struct UIContext {
    pub flow: TextBuffer,
    pub manifest: TextBuffer
    // TODO Log level elector
    // TODO log output with a new logger
    // TODO stdin
    // TODO flow lib path
}

impl UIContext {
    pub fn new() -> Self {
        UIContext {
            flow: TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE),
            manifest: TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE)
        }
    }
}