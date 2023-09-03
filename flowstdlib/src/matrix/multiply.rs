#[doc = include_str!("multiply.md")]

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::Write;

    use tempdir::TempDir;

    use super::super::super::test::execute_flow;

    #[test]
    fn test_multiply_flow() {
        let flow = "\
flow = \"matrix_multiply_test\"

[[process]]
source = \"lib://flowstdlib/matrix/multiply\"
input.a = { once = [[1,2],[3,4]] }
input.b = { once = [[5,6],[7,8]] }

[[process]]
source = \"context://stdio/stdout\"

[[connection]]
from = \"multiply/product\"
to = \"stdout\"
";

        let temp_dir = TempDir::new("flow").expect("Could not create TempDir")
            .into_path();
        let flow_filename = temp_dir.join("matrix_multiply_test.toml");
        let mut flow_file = File::create(&flow_filename)
                .expect("Could not create lib manifest file");
        flow_file.write_all(flow.as_bytes())
            .expect("Could not write data bytes to created flow file");

        let stdout = execute_flow(flow_filename);
        assert_eq!(stdout, "[[19,22],[43,50]]\n".to_string());
    }
}