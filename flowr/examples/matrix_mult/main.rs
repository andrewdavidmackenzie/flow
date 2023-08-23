use utilities;

fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    #[test]
    #[ignore]
    fn test_matrix_mult_example() {
        utilities::test_example(file!(), "flowrcli", false, true);
    }
}