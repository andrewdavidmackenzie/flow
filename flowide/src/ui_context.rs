use flowclib::model::flow::Flow;
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;

pub struct UIContext {
    pub loader: Option<Loader>,
    pub flow: Option<Flow>,
    pub flow_url: Option<String>,
    pub manifest: Option<Manifest>,
    pub manifest_url: Option<String>
}

impl UIContext {
    pub fn new() -> Self {
        UIContext {
            loader: None,
            flow: None,
            flow_url: None,
            manifest: None,
            manifest_url: None
        }
    }
}