use std::fmt;

use model::name::Name;
use model::name::HasName;
use model::datatype::DataType;
use model::datatype::HasDataType;
use model::connection::HasRoute;
use model::io::IO;
use model::connection::Route;
use loader::loader::Validate;
use url::Url;

#[derive(Deserialize, Debug, Clone)]
pub struct Function {
    pub name: Name,

    #[serde(rename = "input")]
    pub inputs: Option<Vec<IO>>,
    #[serde(rename = "output")]
    pub outputs: Option<Vec<IO>>,

    #[serde(skip_deserializing, default = "Function::default_url")]
    pub source_url: Url,

    #[serde(skip_deserializing)]
    pub route: Route,

    #[serde(skip_deserializing)]
    pub lib_reference: Option<String>,
}

impl HasName for Function {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl Validate for Function {
    fn validate(&self) -> Result<(), String> {
        self.name.validate()?;

        let mut io_count = 0;

        if let Some(ref inputs) = self.inputs {
            for i in inputs {
                io_count += 1;
                i.validate()?
            }
        }

        if let Some(ref outputs) = self.outputs {
            for o in outputs {
                io_count += 1;
                o.validate()?
            }
        }

        // A function must have at least one valid input or output
        if io_count == 0 {
            return Err("A function must have at least one input or output".to_string());
        }

        Ok(())
    }
}

#[test]
fn function_with_no_io_not_valid() {
    let fun = Function {
        name: "test_function".to_string(),
        source_url: Function::default_url(),
        inputs: Some(vec!()),
        outputs: Some(vec!()),
        route: "".to_string(),
        lib_reference: None,
    };

    assert_eq!(fun.validate().is_err(), true);
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\t\t\t\t\tname: \t\t{}\n",
               self.name).unwrap();

        write!(f, "\t\t\t\t\t\t\t\tinputs:\n").unwrap();
        if let Some(ref inputs) = self.inputs {
            for input in inputs {
                write!(f, "\t\t\t\t\t\t\t{}\n", input).unwrap();
            }
        }

        write!(f, "\t\t\t\t\t\t\t\toutputs:\n").unwrap();
        if let Some(ref outputs) = self.outputs {
            for output in outputs {
                write!(f, "\t\t\t\t\t\t\t{}\n", output).unwrap();
            }
        }

        Ok(())
    }
}

impl Default for Function {
    fn default() -> Function {
        Function {
            name: "".to_string(),
            inputs: None,
            outputs: None,
            source_url: Function::default_url(),
            route: "".to_string(),
            lib_reference: None,
        }
    }
}

impl Function {
    pub fn default_url() -> Url {
        Url::parse("file:///").unwrap()
    }

    fn get<E: HasName + HasRoute + HasDataType>(&self,
                                                collection: &Option<Vec<E>>,
                                                element_name: &str)
                                                -> Result<(Route, DataType), String> {
        if let &Some(ref elements) = collection {
            for element in elements {
                if element.name() == element_name {
                    return Ok((format!("{}", element.route()), format!("{}", element.datatype())));
                }
            }
            return Err(format!("No output with name '{}' was found", element_name));
        }
        Err(format!("No outputs found."))
    }

    pub fn get_io(&self, direction: &str, name: &Name) -> Result<(Route, DataType), String> {
        match direction {
            "input" => self.get(&self.inputs, name),
            "output" => self.get(&self.outputs, name),
            _ => Err(format!("Count not find {} named '{}' in Function named '{}'",
                             direction, name, self.name))
        }
    }
}