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
    fn test_fibonacci_example() {
        utilities::test_example(file!(), "flowrcli",
                                false, true);
    }
}