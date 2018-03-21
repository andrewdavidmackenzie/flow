use model::name::Name;
use model::name::HasName;
use model::connection::HasRoute;
use model::datatype::DataType;
use model::datatype::HasDataType;
use loader::loader::Validate;
use model::connection::Route;

use std::fmt;

#[derive(Deserialize, Debug, Clone)]
pub struct Input {
    pub name: Name,
    #[serde(rename = "type")]
    pub datatype: DataType,

    #[serde(skip_deserializing)]
    pub route: Route,
}

impl HasName for Input {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl HasDataType for Input {
    fn datatype(&self) -> &str {
        &self.datatype[..]
    }
}

impl HasRoute for Input {
    fn route(&self) -> &str {
        &self.route[..]
    }
}

impl Validate for Input {
    fn validate(&self) -> Result<(), String> {
        self.name.validate()?;
        self.datatype.validate()
    }
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "name: \t\t{}\n\t\t\t\t\troute: \t\t{}\n\t\t\t\t\tdatatype: \t{}\n",
               self.name, self.route, self.datatype)
    }
}

#[cfg(test)]
mod test {
    use toml;
    use super::Input;
    use loader::loader::Validate;
    use model::name::HasName;
    use model::datatype::HasDataType;

    #[test]
    fn deserialize_valid_string_type() {
        let input_str = "\
        name = \"input\"
        type = \"String\"";

        let input: Input = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    fn methods_work() {
        let input_str = "\
        name = \"input\"
        type = \"String\"";

        let input: Input = toml::from_str(input_str).unwrap();
        assert_eq!(input.name(), "input");
        assert_eq!(input.datatype(), "String");
    }

    #[test]
    fn deserialize_valid_json_type() {
        let input_str = "\
        name = \"input\"
        type = \"Json\"";

        let input: Input = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_missing_name() {
        let input_str = "\
        type = \"Json\"";

        let input: Input = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_name_empty() {
        let input_str = "\
        name = \"\"
        type = \"Json\"";

        let input: Input = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_missing_type() {
        let input_str = "\
        name = \"input\"";

        let input: Input = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_unknown_type() {
        let input_str = "\
        name = \"\"
        type = \"Unknown\"";

        let input: Input = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn deserialize_extra() {
        let input_str = "\
        name = \"input\"\
        foo = \"extra token\"
        type = \"Json\"";

        let input: Input = toml::from_str(input_str).unwrap();
        input.validate().unwrap();
    }
}