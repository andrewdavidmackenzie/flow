use implementation::Implementation;
use implementation::RunAgain;
use process::Process;
use runlist::RunList;
use provider::Provider;
use wasmi::{Module};
//use wasmi::{Module, ImportsBuilder, ModuleInstance};
use serde_json::Value as JsonValue;
use url::Url;

pub struct WasmImplementation {
//    module: ModuleInstance
}

impl Implementation for WasmImplementation {
    fn run(&self, _process: &Process, _inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList) -> RunAgain {
        true
    }
}

impl WasmImplementation {
    pub fn load<'a>(provider: &Provider, source_url: &Url) -> Result<&'a Implementation, String> {
        let (resolved_url, _) = provider.resolve(source_url)?;
        let content = provider.get(&resolved_url)?;

        let _module = Module::from_buffer(content)
            .map_err(|e| e.to_string())?;

        Ok(&WasmImplementation {
/*            module: ModuleInstance::new(&module,
                                        &ImportsBuilder::default())
                .map_err(|e| e.to_string())
                .assert_no_start()
                */
        })
    }
}