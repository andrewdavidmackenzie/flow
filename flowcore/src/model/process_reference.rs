use std::collections::BTreeMap;
use std::fmt;

use serde_derive::{Deserialize, Serialize};

use crate::errors::Result;
use crate::model::input::InputInitializer;
use crate::model::name::HasName;
use crate::model::name::Name;
use crate::model::validation::Validate;

/// A `ProcessReference` is the struct used in a `Flow` to refer to a sub-process (Function or nested
/// Flow) it contains
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ProcessReference {
    /// A reference may have an alias - this is used when multiple instances of the same Process
    /// are referenced from within a flow - they need difference aliases to distinguish between them
    /// in connections to/from them
    #[serde(default = "Name::default")]
    pub alias: Name,
    /// Relative or absolute source of the referenced process
    pub source: String,
    /// When a process is references, each reference can set different initial values on the inputs
    /// of the referenced process.
    #[serde(default, rename = "input")]
    pub initializations: BTreeMap<String, InputInitializer>,
    /// Optional X position on the editor canvas (used by flowedit, ignored by flowc).
    /// Accepts both integer and float values in TOML (e.g., `x = 100` or `x = 100.0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f32>,
    /// Optional Y position on the editor canvas (used by flowedit, ignored by flowc)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f32>,
    /// Optional width on the editor canvas (used by flowedit, ignored by flowc)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    /// Optional height on the editor canvas (used by flowedit, ignored by flowc)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
}

impl ProcessReference {
    /// if the `ProcessRef` does not specify an alias for the process to be loaded
    /// then set the alias to be the name of the loaded process
    pub fn set_alias(&mut self, alias: &Name) {
        if self.alias.is_empty() {
            alias.clone_into(&mut self.alias);
        }
    }
}

impl HasName for ProcessReference {
    fn name(&self) -> &Name {
        &self.alias
    }
    fn alias(&self) -> &Name {
        &self.alias
    }
}

impl Validate for ProcessReference {
    fn validate(&self) -> Result<()> {
        self.alias.validate()
    }
}

impl fmt::Display for ProcessReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\t\t\t\talias: {}\n\t\t\t\t\tsource: {}\n\t\t\t\t\tURL: {}\n",
            self.alias, self.source, self.source
        )
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use url::Url;

    use crate::deserializers::deserializer::get;
    use crate::errors::Result;
    use crate::model::input::InputInitializer::{Always, Once};

    use super::ProcessReference;

    fn toml_from_str(content: &str) -> Result<ProcessReference> {
        let url = Url::parse("file:///fake.toml").expect("Could not parse URL");
        let deserializer = get::<ProcessReference>(&url).expect("Could not get deserializer");
        deserializer.deserialize(content, Some(&url))
    }

    #[test]
    fn deserialize_simple() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        ";

        let _reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
    }

    #[test]
    fn deserialize_with_once_input_initialization() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = {once = 1}
        ";

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(
            reference.initializations.len(),
            1,
            "Incorrect number of Input initializations parsed"
        );
        match reference
            .initializations
            .get("input1")
            .expect("Could not get input")
        {
            Always(_) => panic!("Should have been a Once initializer"),
            Once(value) => assert_eq!(&json!(1), value, "input1 should be initialized to 1"),
        }
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
        input.input1 = {always = 1}
        ";

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(
            reference.initializations.len(),
            1,
            "Incorrect number of Input initializations parsed"
        );
        match reference.initializations.get("input1") {
            Some(Always(value)) => {
                assert_eq!(&json!(1), value, "input1 should be initialized to 1");
            }
            _ => panic!("Should have been a Constant initializer"),
        }
    }

    #[test]
    fn deserialize_with_constant_input_initialization_inline_table() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        input.input1 = { always = 1 }
        ";

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(
            reference.initializations.len(),
            1,
            "Incorrect number of Input initializations parsed"
        );
        match reference.initializations.get("input1") {
            Some(Always(value)) => {
                assert_eq!(&json!(1), value, "input1 should be initialized to 1");
            }
            _ => panic!("Should have been an Always initializer"),
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

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(
            reference.initializations.len(),
            2,
            "Incorrect number of Input initializations parsed"
        );
        match reference.initializations.get("input1") {
            Some(Once(value)) => assert_eq!(&json!(1), value, "input1 should be initialized to 1"),
            _ => panic!("Should have been a Once initializer"),
        }

        match reference.initializations.get("input2") {
            Some(Once(value)) => {
                assert_eq!("hello", value, "input2 should be initialized to 'hello'");
            }
            _ => panic!("Should have been a Once initializer"),
        }
    }

    #[test]
    fn deserialize_with_layout_fields() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        x = 100.0
        y = 200.0
        width = 180.0
        height = 120.0
        ";

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(reference.x, Some(100.0));
        assert_eq!(reference.y, Some(200.0));
        assert_eq!(reference.width, Some(180.0));
        assert_eq!(reference.height, Some(120.0));
    }

    #[test]
    fn deserialize_layout_integer_to_float() {
        // Users can write integer values in TOML and serde auto-converts to f32
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        x = 100
        y = 200
        ";

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(reference.x, Some(100.0));
        assert_eq!(reference.y, Some(200.0));
        assert_eq!(reference.width, None);
        assert_eq!(reference.height, None);
    }

    #[test]
    fn deserialize_without_layout_fields() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        ";

        let reference: ProcessReference =
            toml_from_str(input_str).expect("Could not deserialize ProcessReference from toml");
        assert_eq!(reference.x, None);
        assert_eq!(reference.y, None);
        assert_eq!(reference.width, None);
        assert_eq!(reference.height, None);
    }

    #[test]
    fn deserialize_extra_field_fails() {
        let input_str = "
        alias = 'other'
        source = 'other.toml'
        foo = 'extra token'
        ";

        let reference: Result<ProcessReference> = toml_from_str(input_str);
        assert!(reference.is_err());
    }
}
