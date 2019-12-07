use flowrlib::manifest::Manifest;

pub struct UIContext {
    pub manifest: Option<Manifest>,
}

impl UIContext {
    pub fn new() -> Self {
        UIContext {
            manifest: None,
        }
    }
}