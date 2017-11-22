use description::name::Name;
use description::datatype::DataType;

use std::fmt;

#[derive(Deserialize, Debug)]
pub struct Value {
    pub name: Name,
    pub datatype: DataType,
    pub value: String // TODO for now....
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Value:\n\tname: {}\n\tdatatype: {}\n\tvalue: {}", self.name, self.datatype, self.value)
    }
}