use model::name::Name;
use model::name::HasName;
use model::route::Route;
use model::route::HasRoute;
use model::process::Process;
use compiler::loader::Validate;
use std::fmt;
use std::collections::HashMap;
use flowrlib::input::InputInitializer;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ProcessReference {
    pub alias: Name,
    pub source: String,
    #[serde(rename = "input")]
    pub initializations: Option<HashMap<String, InputInitializer>>,
    // Map of initializers of inputs for this reference
    #[serde(skip, default = "ProcessReference::default_url")]
    pub source_url: String,
    #[serde(skip)]
    pub process: Process,
}

impl HasName for ProcessReference {
    fn name(&self) -> &Name { &self.alias }
    fn alias(&self) -> &Name { &self.alias }
}

impl HasRoute for ProcessReference {
    fn route(&self) -> &Route {
        match self.process {
            Process::FlowProcess(ref flow) => {
                flow.route()
            }
            Process::FunctionProcess(ref function) => {
                function.route()
            }
        }
    }
}

impl Validate for ProcessReference {
    fn validate(&self) -> Result<(), String> {
        self.alias.validate()
    }
}

impl fmt::Display for ProcessReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\talias: {}\n\t\t\t\t\tsource: {}\n\t\t\t\t\tURL: {}\n",
               self.alias, self.source, self.source_url)
    }
}

impl ProcessReference {
    fn default_url() -> String {
        "file::///".to_string()
    }
}


#[cfg(test)]
mod test {
    use super::ProcessReference;
    use std::collections::HashMap;
    use model::process::Process;
    use model::function::Function;
    use flowrlib::input::InputInitializer;
    use flowrlib::input::ConstantInputInitializer;
    use flowrlib::input::InputInitializer::{Constant, OneTime};

    #[test]
    fn deserialize_simple() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        ";

        let _reference: ProcessReference = toml::from_str(input_str).unwrap();
    }

    #[test]
    fn deserialize_with_input_initialization() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = {once = 1}
        ";

        let reference: ProcessReference = toml::from_str(input_str).unwrap();
        let initialized_inputs = reference.initializations.unwrap();
        assert_eq!(initialized_inputs.len(), 1, "Incorrect number of Input initializations parsed");
        match initialized_inputs.get("input1").unwrap() {
            OneTime(one_time) => assert_eq!(1, one_time.once, "input1 should be initialized to 1"),
            Constant(_) => panic!("Should have been a OneTime initializer")
        }
    }

    /*
        The serializer chooses the other form of table output, not the 'inline table' I use
        generally for input, but it's still valid
    */
    #[test]
    fn serialize_with_constant_input_initialization() {
        let expected = "alias = 'other'
source = 'other.toml'
[input.input1]
constant = 1
";

        let constant_initializer = ConstantInputInitializer {
            constant: json!(1)
        };
        let input_initializer = super::InputInitializer::Constant(constant_initializer);
        let mut initializers = HashMap::<String, InputInitializer>::new();
        initializers.insert("input1".to_string(), input_initializer);
        let function = Function::new("function".to_string(), true,
                                     None, "alias".to_string(),
                                     None, None, "url".to_string(),
                                     "route".to_string(), None, vec!(), 0);
        let reference = ProcessReference {
            alias: "other".to_string(),
            source: "other.toml".to_string(),
            initializations: Some(initializers),
            source_url: "don't care".to_string(),
            process: Process::FunctionProcess(function),
        };

        let actual = toml::to_string(&reference).unwrap();

        assert_eq!(expected.replace("'", "\""), actual);
    }

    /*
        For completeness I test the alternative format of expressing the table, but I prefer to use
        and will document the inline table that is tested below.
    */
    #[test]
    fn deserialize_with_constant_input_initialization() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        [input.input1]
        constant = 1
        ";

        let reference: ProcessReference = toml::from_str(input_str).unwrap();
        let initialized_inputs = reference.initializations.unwrap();
        assert_eq!(initialized_inputs.len(), 1, "Incorrect number of Input initializations parsed");
        match initialized_inputs.get("input1").unwrap() {
            OneTime(one_time) => {
                println!("initial_value: {}", one_time.once);
                panic!("Should have been a Constant initializer")
            },
            Constant(constant) => {
                assert_eq!(1, constant.constant, "input1 should be initialized to 1");
            }
        }
    }

    #[test]
    fn deserialize_with_constant_input_initialization_inline_table() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = { constant = 1 }
        ";

        let reference: ProcessReference = toml::from_str(input_str).unwrap();
        let initialized_inputs = reference.initializations.unwrap();
        assert_eq!(initialized_inputs.len(), 1, "Incorrect number of Input initializations parsed");
        match initialized_inputs.get("input1").unwrap() {
            OneTime(_) => panic!("Should have been a Constant initializer"),
            Constant(constant) => {
                assert_eq!(1, constant.constant, "input1 should be initialized to 1");
            }
        }
    }

    #[test]
    fn deserialize_with_multiple_input_initialization() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = {once = 1}
        input.input2 = {once = 'hello'}
        ";

        let reference: ProcessReference = toml::from_str(input_str).unwrap();
        let initialized_inputs = reference.initializations.unwrap();
        assert_eq!(initialized_inputs.len(), 2, "Incorrect number of Input initializations parsed");
        match initialized_inputs.get("input1").unwrap() {
            OneTime(one_time) => assert_eq!(1, one_time.once, "input1 should be initialized to 1"),
            _ => panic!("Should have been a simple initializer")
        }

        match initialized_inputs.get("input2").unwrap() {
            OneTime(one_time) => assert_eq!("hello", one_time.once, "input2 should be initialized to 'hello'"),
            _ => panic!("Should have been a simple initializer")
        }
    }

    #[test]
    #[should_panic]
    fn deserialize_extra_field_fails() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        foo = 'extra token'
        ";

        let _reference: ProcessReference = toml::from_str(input_str).unwrap();
    }
}