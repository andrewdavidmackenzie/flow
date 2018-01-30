extern crate flowclib;
extern crate url;

mod test {
    use url::Url;
    use std::env;
    use flowclib::loader::loader;
    use flowclib::dumper::dumper;

    fn url_from_rel_path(path: &str) -> Url {
        let parent = Url::from_file_path(env::current_dir().unwrap()).unwrap();
        parent.join(path).unwrap()
    }

    #[test]
    fn dump_hello_world_simple() {
        let flow = loader::load(&url_from_rel_path("samples/hello-world-simple/context.toml")).unwrap();
        dumper::dump(&flow);
    }

    #[test]
    fn dump_hello_world_context() {
        let flow = loader::load(&url_from_rel_path("samples/hello-world/context.toml")).unwrap();
        dumper::dump(&flow);
    }

    #[test]
    fn dump_hello_world_include() {
        loader::load(&url_from_rel_path("samples/hello-world-include/context.toml")).unwrap();
    }

    #[test]
    fn dump_hello_world_flow1() {
        loader::load(&url_from_rel_path("samples/hello-world/flow1.toml")).unwrap();
    }

    #[test]
    fn dump_complex1() {
        loader::load(&url_from_rel_path("samples/complex1/context.toml")).unwrap();
    }

    #[test]
    fn dump_fibonacci() {
        loader::load(&url_from_rel_path("samples/fibonacci/context.toml")).unwrap();
    }
}