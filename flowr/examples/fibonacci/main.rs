extern crate serial_test;

use utilities;

fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_fibonacci_example() {
        utilities::test_example(file!(), "flowrcli", false, true, false);
    }

    #[test]
    #[serial]
    fn test_fibonacci_wasm_example() {
        utilities::test_example(file!(), "flowrcli", false, false, false);
    }

    #[test]
    fn test_fibonacci_flowrex_example() {
        utilities::test_example(file!(), "flowrcli", true, true, false);
    }
}