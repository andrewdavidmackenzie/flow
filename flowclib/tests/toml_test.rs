extern crate flowclib;
extern crate url;

mod test {
    use url::Url;
    use std::env;
    use flowclib::loader::loader;

    fn url_from_rel_path(path: &str) -> Url {
        let parent = Url::from_file_path(env::current_dir().unwrap()).unwrap();
        parent.join(path).unwrap()
    }

    #[test]
    fn load_hello_world_simple_from_context() {
        loader::load(&"hello-world-simple".to_string(), &url_from_rel_path("samples/hello-world-simple/context.toml")).unwrap();
    }

    #[test]
    fn load_hello_world_from_context() {
        loader::load(&"hello-world".to_string(), &url_from_rel_path("samples/hello-world/context.toml")).unwrap();
    }

    #[test]
    fn load_hello_world_include() {
        loader::load(&"hello-world-include".to_string(), &url_from_rel_path("samples/hello-world-include/context.toml")).unwrap();
    }

    #[test]
    fn load_hello_world_flow1() {
        loader::load(&"flow1".to_string(), &url_from_rel_path("samples/hello-world/flow1.toml")).unwrap();
    }

    #[test]
    fn load_reverse_echo_from_toml() {
        loader::load(&"reverse-echo".to_string(), &url_from_rel_path("samples/reverse-echo/context.toml")).unwrap();
    }

    #[test]
    fn load_fibonacci_from_toml() {
        loader::load(&"fibonacci".to_string(), &url_from_rel_path("samples/fibonacci/context.toml")).unwrap();
    }

    #[test]
    fn load_fibonacci_from_directory() {
        loader::load(&"fibonacci".to_string(), &url_from_rel_path("samples/fibonacci")).unwrap();
    }
}