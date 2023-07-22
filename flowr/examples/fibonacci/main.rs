use utilities;

fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    #[test]
    fn test_fibonacci_example() {
        utilities::test_example(file!(), "flowrcli", false, true);
    }

    /*
    #[test]
    fn test_fibonacci_wasm_example() {
        utilities::test_example(file!(), "flowrcli", false, false);
    }

    #[test]
    fn test_fibonacci_flowrex_example() {
        utilities::test_example(file!(), "flowrcli", true, true);
    }

     */
}