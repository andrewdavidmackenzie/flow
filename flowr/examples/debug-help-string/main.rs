use utilities;

fn main() {
    #[cfg(feature = "debugger")]
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    #[cfg(feature = "debugger")]
    #[test]
    fn test_debug_help_string_example() {
        utilities::test_example(file!(), "flowrcli", false, true);
    }
}