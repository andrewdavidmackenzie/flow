use std::path::PathBuf;

use utilities;

fn main() {
    let mut sample_dir = PathBuf::from(file!());
    sample_dir.pop();
    utilities::run_sample(&sample_dir, false, true).unwrap();
}


#[cfg(test)]
mod test {
    #[test]
    fn test_hello_world_example() {
        let mut sample_dir = PathBuf::from(file!());
        sample_dir.pop();
        utilities::test_example(&sample_dir, false, true);
    }
}