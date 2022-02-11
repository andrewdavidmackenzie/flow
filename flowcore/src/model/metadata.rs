
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Debug, Default, Serialize, PartialEq)]
/// `MetaData` about a `flow` that will be used in the flow's description and `Manifest`
pub struct MetaData {
    /// The human readable `name` of a `flow`
    #[serde(default)]
    pub name: String,
    /// Semantic versioning version number of the flow
    #[serde(default)]
    pub version: String,
    /// A description for humans
    #[serde(default)]
    pub description: String,
    /// The name of the people who authored the flow
    #[serde(default)]
    pub authors: Vec<String>,
}