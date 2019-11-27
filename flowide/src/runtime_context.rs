use gtk::TextBuffer;

/// `RuntimeCOntext` Holds items from the UI that are needed during runnings of a slow.
pub struct RuntimeContext {
    pub args: TextBuffer,
    pub stdout: TextBuffer,
    pub stderr: TextBuffer,
}

impl RuntimeContext {
    pub fn new(args: TextBuffer, stdout: TextBuffer, stderr: TextBuffer) -> Self {
        RuntimeContext {
            args,
            stdout,
            stderr,
        }
    }
}