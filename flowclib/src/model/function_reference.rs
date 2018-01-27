use std::fmt;

use model::name::Name;
use model::name::HasName;
use model::connection::Route;
use model::connection::HasRoute;
use model::datatype::DataType;
use model::datatype::HasDataType;
use model::function::Function;
use loader::loader::Validate;
use url::Url;

// This structure is (optionally) found as part of a flow file - inline in the description
#[derive(Deserialize, Debug)]
pub struct FunctionReference {
    pub alias: Name,
    pub source: String,
    #[serde(skip_deserializing, default = "FunctionReference::default_url")]
    pub source_url: Url,
    #[serde(skip_deserializing)]
    pub function: Function,
}

impl HasName for FunctionReference {
    fn name(&self) -> &str {
        &self.alias[..]
    }
}

impl HasRoute for FunctionReference {
    fn route(&self) -> &str {
        &self.function.route[..]
    }
}

impl Validate for FunctionReference {
    fn validate(&self) -> Result<(), String> {
        self.alias.validate()
        // TODO validate source
    }
}

impl fmt::Display for FunctionReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\t\talias: \t{}\n\t\t\t\t\timplementation:\n\t\t\t\t\t\t\t\tsource: \t{}\n",
               self.alias, self.source)
    }
}

// TODO see if can de-duplicate code from flow reference and function reference
impl FunctionReference {
    fn default_url() -> Url {
        Url::parse("file:///").unwrap()
    }

    fn get<E: HasName + HasRoute + HasDataType>(&self,
                                                collection: &Option<Vec<E>>,
                                                element_name: &str)
                                                -> Result<(Route, DataType, bool), String> {
        if let &Some(ref elements) = collection {
            for element in elements {
                if element.name() == element_name {
                    return Ok((format!("{}", element.route()),
                               format!("{}", element.datatype()),
                               false));
                }
            }
            return Err(format!("No element with name '{}' was found", element_name));
        }
        Err(format!("No elements found."))
    }

    pub fn get_io(&self, direction: &str, name: &str) -> Result<(Route, DataType, bool), String> {
        match direction {
            "input" => self.get(&self.function.inputs, name),
            "output" => self.get(&self.function.outputs, name),
            _ => return Err(format!("Invalid name '{}' used in connection", name))
        }
    }
}