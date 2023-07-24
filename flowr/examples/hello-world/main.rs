extern crate serial_test;

use utilities;

fn main() {
    utilities::run_example(file!(), "flowrcli",
                           false, true);
}

#[cfg(test)]
mod test {
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_hello_world_example() {
        utilities::test_example(file!(), "flowrcli",
                                false, true);
    }

    /*

    #[test]
    #[serial]
    fn hello_world_client_server() {
        let manifest = compile_and_execute("flowrcli",
                                           "hello-world",
                                           false,
                                           false,
                                           false,)
            .expect("Test failed");

        execute_flow_client_server("hello-world", manifest)
            .expect("Client/Server execution failed");
    }

     */

}