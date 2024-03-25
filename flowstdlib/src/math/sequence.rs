#[doc = include_str!("sequence.md")]

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::Write;

    use tempfile::tempdir;

    use super::super::super::test::execute_flow;

    #[test]
    fn test_sequence_flow() {
        let flow = "\
flow = \"sequence_test\"

[[process]]
source = \"lib://flowstdlib/math/sequence\"
input.start = { once = 1 }
input.limit = { once = 10 }
input.step = { once = 1 }

[[process]]
source = \"context://stdio/stdout\"

[[connection]]
from = \"sequence/number\"
to = \"stdout\"
";

        let temp_dir = tempdir().expect("Could not create temporary directory").into_path();
        let flow_filename = temp_dir.join("sequence_test.toml");
        let mut flow_file =
            File::create(&flow_filename).expect("Could not create lib manifest file");
        flow_file.write_all(flow.as_bytes()).expect("Could not write data bytes to created flow file");

        let stdout = execute_flow(&flow_filename);

        let numbers: Vec<i32> = stdout.lines().map(|l| l.parse::<i32>().expect("Not a number")).collect::<Vec<i32>>();
        assert_eq!(numbers, vec!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10));
    }

}