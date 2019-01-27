
use ::generator::generate::GenerationTables;

pub fn build_functions(_out_dir_path: &str, tables: &GenerationTables) -> Result<String, String> {
    for runnable in &tables.runnables {
        if let Some(_source) = runnable.get_source_url() {

        }
    }

    Ok("jobs".to_string())
}