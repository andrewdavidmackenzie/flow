use model::name::HasName;
use model::connection::HasRoute;
use model::datatype::HasDataType;
use model::datatype::DataType;
use model::datatype::TypeCheck;
use loader::loader::Validate;
use model::connection::Route;

use std::fmt;

#[derive(Deserialize, Debug, Clone)]
pub struct IO {
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(rename = "type", default = "default_type")]
    pub datatype: DataType,

    #[serde(skip_deserializing)]
    pub route: Route,
}

pub type IOSet = Option<Vec<IO>>;

impl HasName for IO {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl HasDataType for IO {
    fn datatype(&self) -> &str {
        &self.datatype[..]
    }
}

impl HasRoute for IO {
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

impl Validate for IO {
    fn validate(&self) -> Result<(), String> {
        self.datatype.valid()
    }
}

impl Validate for IOSet {
    fn validate(&self) -> Result<(), String> {
        if let &Some(ref ios) = self {
            for io in ios {
                io.validate()?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for IO {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "name: \t\t{}\n\t\t\t\t\troute: \t\t{}\n\t\t\t\t\tdatatype: \t{}\n",
               self.name, self.route, self.datatype)
    }
}

#[cfg(test)]
mod test {
    use toml;
    use super::IO;
    use loader::loader::Validate;
    use model::name::HasName;
    use model::datatype::HasDataType;

    #[test]
    fn deserialize_empty_string() {
        let input_str = "";

        let output: IO = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
        assert_eq!(output.datatype, "Json");
        assert_eq!(output.name, "");
    }

    #[test]
    fn deserialize_valid_type() {
        let input_str = "\
        type = \"String\"";

        let output: IO = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_invalid_type() {
        let input_str = "\
        type = \"Unknown\"";

        let output: IO = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
    }

    #[test]
    fn deserialize_name() {
        let input_str = "\
        name = \"/sub_route\"
        type = \"String\"";

        let output: IO = toml::from_str(input_str).unwrap();
        output.validate().unwrap();
        assert_eq!(output.name, "/sub_route");
    }




    #[test]
    fn deserialize_valid_string_type() {
        let input_str = "\
        name = \"input\"
        type = \"String\"";

        let input: IO = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    fn methods_work() {
        let input_str = "\
        name = \"input\"
        type = \"String\"";

        let input: IO = toml::from_str(input_str).unwrap();
        assert_eq!(input.name(), "input");
        assert_eq!(input.datatype(), "String");
    }

    #[test]
    fn deserialize_valid_json_type() {
        let input_str = "\
        name = \"input\"
        type = \"Json\"";

        let input: IO = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_extra() {
        let input_str = "\
        name = \"input\"\
        foo = \"extra token\"
        type = \"Json\"";

        let input: IO = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }
}