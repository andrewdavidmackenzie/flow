use serde_derive::{Deserialize, Serialize};

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::name::Name;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Library {
    #[serde(rename = "library")]
    pub name: Name,
    #[serde(default = "Library::default_description")]
    pub description: String,
    #[serde(default = "Library::default_version")]
    pub version: String,
    #[serde(default = "Library::default_author")]
    pub author_name: String,
    #[serde(default = "Library::default_email")]
    pub author_email: String,
}

impl Validate for Library {
    fn validate(&self) -> Result<()> {
        self.name.validate()
    }
}

impl Default for Library {
    fn default() -> Library {
        Library {
            name: Name::default(),
            description: Library::default_description(),
            version: Library::default_version(),
            author_name: Library::default_author(),
            author_email: Library::default_email(),
        }
    }
}

impl Library {
    pub fn default_description() -> String {
        "".into()
    }
    pub fn default_version() -> String {
        "0.0.0".to_string()
    }
    pub fn default_author() -> String {
        "unknown".to_string()
    }
    pub fn default_email() -> String {
        "unknown@unknown.com".to_string()
    }
}