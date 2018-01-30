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
    #[ignore]
    fn sample_hello_world_simple_yaml() {
        loader::load(&url_from_rel_path("samples/hello-world-simple-yaml/context.yaml")).unwrap();
    }

    #[test]
    #[ignore]
    fn sample_hello_world_yaml() {
        loader::load(&url_from_rel_path("samples/hello-world-yaml/context.yaml")).unwrap();
    }
}