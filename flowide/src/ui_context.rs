use std::sync::{Arc, Mutex};

use flowclib::model::flow::Flow;
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;
use flowrlib::runtime_client::RuntimeClient;

use crate::ide_runtime_client::IDERuntimeClient;

pub struct UIContext {
    pub loader: Option<Loader>,
    pub flow: Option<Flow>,
    pub flow_url: Option<String>,
    pub manifest: Option<Manifest>,
    pub manifest_url: Option<String>,
    pub client: Arc<Mutex<dyn RuntimeClient>>,
}

impl UIContext {
    pub fn new() -> Self {
        UIContext {
            loader: None,
            flow: None,
            flow_url: None,
            manifest: None,
            manifest_url: None,
            client: Arc::new(Mutex::new(IDERuntimeClient::new())),
        }
    }
}