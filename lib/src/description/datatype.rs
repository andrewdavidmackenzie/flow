const DATATYPES: &'static [&'static str] = &["String"];

pub struct DataType;

impl DataType {
    pub fn valid_type(datatype: &str) -> Result<(), String> {
        if DATATYPES.contains(&datatype) {
            return Ok(());
        }

        Err(format!("DataType '{}' is unknown", datatype))
    }
}

#[test]
fn valid_data_type() {
    DataType::valid_type("String").unwrap();
}

#[test]
#[should_panic]
fn invalid_data_type() {
    DataType::valid_type("foo").unwrap();
}
