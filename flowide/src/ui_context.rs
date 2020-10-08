use flowclib::model::flow::Flow;
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;

use crate::ide_runtime_client::IDERuntimeClient;

pub struct UIContext {
    pub loader: Option<Loader>,
    pub flow: Option<Flow>,
    pub flow_url: Option<String>,
    pub manifest: Option<Manifest>,
    pub manifest_url: Option<String>,
    pub client: IDERuntimeClient
}

impl UIContext {
    pub fn new() -> Self {
        UIContext {
            loader: None,
            flow: None,
            flow_url: None,
            manifest: None,
            manifest_url: None,
            client: IDERuntimeClient::new()
        }
    }
}