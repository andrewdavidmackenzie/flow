fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    #[test]
    fn test_json_indexing_example() {
        utilities::test_example(file!(), "flowrcli", false, true);
    }
}
