use utilities;

fn main() {
    utilities::run_sample(&file!(), false, true).unwrap();
}


#[cfg(test)]
mod test {
    #[test]
    fn test_hello_world_example() {
        utilities::test_example(file!(), false, true);
    }
}