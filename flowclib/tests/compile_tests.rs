extern crate flowclib;
extern crate url;

mod test {
    use url::Url;
    use std::env;
    use flowclib::loader::loader;
    use flowclib::compiler::compile;

    fn url_from_rel_path(path: &str) -> Url {
        let parent = Url::from_file_path(env::current_dir().unwrap()).unwrap();
        parent.join(path).unwrap()
    }

    #[test]
    #[should_panic]
    fn compiled_detects_competing_inputs() {
        let mut flow =  loader::load(&"competing".to_string(), &url_from_rel_path("flowclib/tests/competing.toml")).unwrap();
        let _tables = compile::compile(&mut flow).unwrap();
    }

    #[test]
    #[should_panic]
    fn compiler_detects_loop() {
        let mut flow =  loader::load(&"loop".to_string(), &url_from_rel_path("flowclib/tests/loop.toml")).unwrap();
        let _tables = compile::compile(&mut flow);
    }
}