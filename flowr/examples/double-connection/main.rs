fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    #[test]
    fn test_double_connection_example() {
        utilities::test_example(file!(), "flowrcli", false, true);
    }
}
