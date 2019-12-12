use flowclib::model::flow::Flow;
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;

pub struct UIContext {
    pub loader: Option<Loader>,
    pub flow: Option<Flow>,
    pub manifest: Option<Manifest>
}

impl UIContext {
    pub fn new() -> Self {
        UIContext {
            loader: None,
            flow: None,
            manifest: None,
        }
    }
}