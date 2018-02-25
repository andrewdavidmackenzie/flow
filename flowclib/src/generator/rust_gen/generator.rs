use generator::rust_gen::cargo_gen;
use generator::rust_gen::functions_gen;
use generator::rust_gen::runnables_gen;
use generator::rust_gen::main_gen;
use std::collections::HashMap;
use std::path::PathBuf;
use compiler::compile::CompilerTables;
use std::fs;
use std::io::Result;
use super::super::code_gen::CodeGenerator;

pub struct RustGenerator;

impl CodeGenerator for RustGenerator {
    fn generate(&self, output_dir: &PathBuf, mut vars: &mut HashMap<String, &str>, tables: &CompilerTables)
                -> Result<((String, Vec<String>), (String, Vec<String>))> {
        let ((build, build_args), (run, run_args)) =
            cargo_gen::create(&output_dir, &vars)?;
        let src_dir = RustGenerator::create_src_dir(&output_dir)?;
        functions_gen::copy(&src_dir, &tables)?;
        main_gen::create(&src_dir, &mut vars, tables)?;
        runnables_gen::create(&src_dir, tables)?;
        Ok(((build, build_args), (run, run_args)))
    }
}

impl RustGenerator {
    fn create_src_dir(root: &PathBuf) -> Result<PathBuf> {
        let mut dir = root.clone();
        dir.push("src");
        if !dir.exists() {
            fs::create_dir(&dir)?;
        }
        Ok(dir)
    }
}