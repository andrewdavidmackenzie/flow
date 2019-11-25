use gtk::TextBuffer;

/// `RuntimeCOntext` Holds items from the UI that are needed during runnings of a slow.
pub struct RuntimeContext<'a> {
    pub args: &'a TextBuffer,
    pub stdout: &'a TextBuffer,
    pub stderr: &'a TextBuffer,
}

impl<'a> RuntimeContext<'a> {
    pub fn new(args: &'a TextBuffer, stdout: &'a TextBuffer, stderr: &'a TextBuffer) -> Self {
        RuntimeContext {
            args,
            stdout,
            stderr,
        }
    }
}