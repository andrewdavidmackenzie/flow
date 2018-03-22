use model::name::HasName;
use model::connection::HasRoute;
use model::datatype::HasDataType;
use model::datatype::DataType;
use model::datatype::TypeCheck;
use loader::loader::Validate;
use model::connection::Route;

use std::fmt;

// TODO ADM combine input and output again

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Output {
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(rename = "type", default = "default_type")]
    pub datatype: DataType,

    // will be the path to the value or function that has the output
    #[serde(skip_deserializing)]
    pub route: Route,
}

impl HasName for Output {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl HasDataType for Output {
    fn datatype(&self) -> &str {
        &self.datatype[..]
    }
}

impl HasRoute for Output {
    fn route(&self) -> &str {
        &self.route[..]
    }
}

fn default_name() -> String {
    "".to_string()
}

fn default_type() -> String {
    "Json".to_string()
}

impl Validate for Output {
    fn validate(&self) -> Result<(), String> {
        self.datatype.valid()
    }
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\t\troute: \t\t{}\n", self.route)?;
        write!(f, "\t\t\t\t\tdatatype: \t{}\n", self.datatype)
    }
}

#[cfg(test)]
mod test {
    use toml;
    use super::Output;
    use loader::loader::Validate;

    #[test]
    fn deserialize_empty_string() {
        let input_str = "";

        let output: Output = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
        assert_eq!(output.datatype, "Json");
        assert_eq!(output.name, "");
    }

    #[test]
    fn deserialize_valid_type() {
        let input_str = "\
        type = \"String\"";

        let output: Output = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_invalid_type() {
        let input_str = "\
        type = \"Unknown\"";

        let output: Output = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
    }

    #[test]
    fn deserialize_name() {
        let input_str = "\
        name = \"/sub_route\"
        type = \"String\"";

        let output: Output = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
        assert_eq!(output.name, "/sub_route");
    }
}