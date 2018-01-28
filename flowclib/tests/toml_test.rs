extern crate flowclib;
extern crate url;

#[cfg(test)]
mod test {
    use url::Url;
    use std::env;
    use flowclib::loader::loader::load;

    fn url_from_rel_path(path: &str) -> Url {
        let parent = Url::from_file_path(env::current_dir().unwrap()).unwrap();
        parent.join(path).unwrap()
    }

    #[test]
    fn dump_hello_world_simple() {
        load(&url_from_rel_path("samples/hello-world-simple/context.toml"), true).unwrap();
    }

    #[test]
    fn dump_hello_world_context() {
        load(&url_from_rel_path("samples/hello-world/context.toml"), true).unwrap();
    }

    #[test]
    fn dump_hello_world_include() {
        load(&url_from_rel_path("samples/hello-world-include/context.toml"), true).unwrap();
    }

    #[test]
    fn dump_hello_world_flow1() {
        load(&url_from_rel_path("samples/hello-world/flow1.toml"), true).unwrap();
    }

    #[test]
    fn dump_complex1() {
        load(&url_from_rel_path("samples/complex1/context.toml"), true).unwrap();
    }

    #[test]
    fn dump_fibonacci() {
        load(&url_from_rel_path("samples/fibonacci/context.toml"), true).unwrap();
    }
}