use gtk::TextBuffer;

/// `RuntimeContext` Holds items from the UI that are needed during runnings of a flow. They will
/// be written to by the UI Thread via the ui_runtime_client
#[derive(Clone)]
pub struct RuntimeContext {
    pub args: TextBuffer,
    pub stdout: TextBuffer,
    pub stderr: TextBuffer,
}

impl RuntimeContext {
    pub fn new() -> Self {
        RuntimeContext {
            args: TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE),
            stdout: TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE),
            stderr: TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE),
        }
    }
}