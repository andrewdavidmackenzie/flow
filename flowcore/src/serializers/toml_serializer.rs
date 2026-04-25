//! TOML serialization for flow and function definitions.
//!
//! Hand-built TOML output that matches the format expected by the flow
//! deserializer. The derived `Serialize` on some flowcore types produces
//! struct-style output that is not compatible with the flow format.

use std::fmt::Write;
use std::path::Path;

use crate::model::flow_definition::FlowDefinition;
use crate::model::function_definition::FunctionDefinition;
use crate::model::input::InputInitializer;
use crate::model::io::IO;
use crate::model::name::HasName;

fn escape_toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            other => out.push(other),
        }
    }
    out
}

/// Serialize a `serde_json::Value` into a TOML-compatible inline value string.
pub fn value_to_toml(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => {
            format!("\"{}\"", escape_toml_string(s))
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "\"null\"".to_string(),
        serde_json::Value::Array(a) => {
            let items: Vec<String> = a.iter().map(value_to_toml).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(m) => {
            let items: Vec<String> = m
                .iter()
                .map(|(k, val)| format!("{k} = {}", value_to_toml(val)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
    }
}

fn initializer_to_toml(init: &InputInitializer) -> String {
    match init {
        InputInitializer::Once(v) => format!("{{ once = {} }}", value_to_toml(v)),
        InputInitializer::Always(v) => format!("{{ always = {} }}", value_to_toml(v)),
    }
}

fn write_io_toml(out: &mut String, section: &str, io: &IO) {
    let _ = writeln!(out, "\n[[{section}]]");
    let name = io.name();
    if !name.is_empty() {
        let _ = writeln!(out, "name = \"{name}\"");
    }
    let types = io.datatypes();
    if types.len() == 1 {
        if let Some(t) = types.first() {
            let _ = writeln!(out, "type = \"{t}\"");
        }
    } else if types.len() > 1 {
        let ts: Vec<String> = types.iter().map(|t| format!("\"{t}\"")).collect();
        let _ = writeln!(out, "type = [{}]", ts.join(", "));
    }
}

impl FlowDefinition {
    /// Serialize this flow definition to TOML format.
    #[must_use]
    pub fn to_toml(&self) -> String {
        let mut out = String::new();

        let _ = writeln!(out, "flow = \"{}\"", escape_toml_string(&self.name));

        if !self.description.is_empty() {
            let _ = writeln!(
                out,
                "description = \"{}\"",
                escape_toml_string(&self.description)
            );
        }

        if !self.docs.is_empty() {
            let _ = writeln!(out, "docs = \"{}\"", escape_toml_string(&self.docs));
        }

        let md = &self.metadata;
        if !md.version.is_empty() || !md.description.is_empty() || !md.authors.is_empty() {
            out.push_str("\n[metadata]\n");
            if !md.version.is_empty() {
                let _ = writeln!(out, "version = \"{}\"", escape_toml_string(&md.version));
            }
            if !md.description.is_empty() {
                let _ = writeln!(
                    out,
                    "description = \"{}\"",
                    escape_toml_string(&md.description)
                );
            }
            if !md.authors.is_empty() {
                let authors: Vec<String> = md
                    .authors
                    .iter()
                    .map(|a| format!("\"{}\"", escape_toml_string(a)))
                    .collect();
                let _ = writeln!(out, "authors = [{}]", authors.join(", "));
            }
        }

        for input in &self.inputs {
            write_io_toml(&mut out, "input", input);
        }

        for output in &self.outputs {
            write_io_toml(&mut out, "output", output);
        }

        for pref in &self.process_refs {
            out.push_str("\n[[process]]\n");
            if !pref.alias.is_empty() {
                let _ = writeln!(out, "alias = \"{}\"", pref.alias);
            }
            let _ = writeln!(out, "source = \"{}\"", pref.source);

            if let Some(x) = pref.x {
                let _ = writeln!(out, "x = {x}");
            }
            if let Some(y) = pref.y {
                let _ = writeln!(out, "y = {y}");
            }
            if let Some(w) = pref.width {
                let _ = writeln!(out, "width = {w}");
            }
            if let Some(h) = pref.height {
                let _ = writeln!(out, "height = {h}");
            }

            for (port_name, init) in &pref.initializations {
                let _ = writeln!(out, "input.{port_name} = {}", initializer_to_toml(init));
            }
        }

        for conn in &self.connections {
            let _ = writeln!(out, "\n[[connection]]");
            if !conn.name().is_empty() {
                let _ = writeln!(out, "name = \"{}\"", conn.name());
            }
            let _ = writeln!(out, "from = \"{}\"", conn.from());
            if let [single] = conn.to().as_slice() {
                let _ = writeln!(out, "to = \"{single}\"");
            } else {
                let to_strs: Vec<String> = conn.to().iter().map(|r| format!("\"{r}\"")).collect();
                let _ = writeln!(out, "to = [{}]", to_strs.join(", "));
            }
        }

        out
    }

    /// Save this flow definition to a TOML file at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.to_toml()).map_err(|e| format!("Could not write file: {e}"))
    }
}

impl FunctionDefinition {
    /// Serialize this function definition to TOML format.
    #[must_use]
    pub fn to_toml(&self) -> String {
        let mut out = format!(
            "function = \"{}\"\nsource = \"{}\"\ntype = \"rust\"\n",
            escape_toml_string(&self.name),
            escape_toml_string(&self.source)
        );

        if !self.description.is_empty() {
            let _ = writeln!(
                out,
                "description = \"{}\"",
                escape_toml_string(&self.description)
            );
        }

        for input in &self.inputs {
            write_io_toml(&mut out, "input", input);
        }

        for output in &self.outputs {
            write_io_toml(&mut out, "output", output);
        }

        out
    }

    /// Save this function definition to a TOML file at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.to_toml()).map_err(|e| format!("Could not write file: {e}"))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::connection::Connection;
    use crate::model::datatype::DataType;
    use crate::model::io::IO;
    use crate::model::metadata::MetaData;
    use crate::model::name::Name;
    use crate::model::process_reference::ProcessReference;
    use crate::model::route::Route;
    use std::collections::BTreeMap;

    #[test]
    fn flow_to_toml_minimal() {
        let flow = FlowDefinition {
            name: Name::from("test"),
            ..FlowDefinition::default()
        };
        let toml = flow.to_toml();
        assert!(toml.contains("flow = \"test\""));
    }

    #[test]
    fn flow_to_toml_with_description() {
        let flow = FlowDefinition {
            name: Name::from("test"),
            description: "A test flow".into(),
            ..FlowDefinition::default()
        };
        let toml = flow.to_toml();
        assert!(toml.contains("description = \"A test flow\""));
    }

    #[test]
    fn flow_to_toml_with_metadata() {
        let flow = FlowDefinition {
            name: Name::from("test"),
            metadata: MetaData {
                name: String::new(),
                version: "1.0.0".into(),
                description: String::new(),
                authors: vec!["Author".into()],
            },
            ..FlowDefinition::default()
        };
        let toml = flow.to_toml();
        assert!(toml.contains("[metadata]"));
        assert!(toml.contains("version = \"1.0.0\""));
        assert!(toml.contains("authors = [\"Author\"]"));
    }

    #[test]
    fn flow_to_toml_with_io() {
        let flow = FlowDefinition {
            name: Name::from("test"),
            inputs: vec![IO::new_named(
                vec![DataType::from("string")],
                Route::default(),
                "in0",
            )],
            outputs: vec![IO::new_named(
                vec![DataType::from("number")],
                Route::default(),
                "out0",
            )],
            ..FlowDefinition::default()
        };
        let toml = flow.to_toml();
        assert!(toml.contains("[[input]]"));
        assert!(toml.contains("name = \"in0\""));
        assert!(toml.contains("[[output]]"));
        assert!(toml.contains("name = \"out0\""));
    }

    #[test]
    fn flow_to_toml_with_process() {
        let flow = FlowDefinition {
            name: Name::from("test"),
            process_refs: vec![ProcessReference {
                alias: Name::from("add"),
                source: "lib://flowstdlib/math/add".into(),
                initializations: BTreeMap::new(),
                x: Some(100.0),
                y: Some(200.0),
                width: None,
                height: None,
            }],
            ..FlowDefinition::default()
        };
        let toml = flow.to_toml();
        assert!(toml.contains("[[process]]"));
        assert!(toml.contains("alias = \"add\""));
        assert!(toml.contains("source = \"lib://flowstdlib/math/add\""));
        assert!(toml.contains("x = 100"));
        assert!(toml.contains("y = 200"));
    }

    #[test]
    fn flow_to_toml_with_connection() {
        let flow = FlowDefinition {
            name: Name::from("test"),
            connections: vec![Connection::new("input/string", "print")],
            ..FlowDefinition::default()
        };
        let toml = flow.to_toml();
        assert!(toml.contains("[[connection]]"));
        assert!(toml.contains("from = \"input/string\""));
        assert!(toml.contains("to = \"print\""));
    }

    #[test]
    fn flow_to_toml_with_initializer() {
        let mut inits = BTreeMap::new();
        inits.insert(
            "start".into(),
            InputInitializer::Once(serde_json::json!(42)),
        );
        let flow = FlowDefinition {
            name: Name::from("test"),
            process_refs: vec![ProcessReference {
                alias: Name::from("seq"),
                source: "lib://flowstdlib/control/tap".into(),
                initializations: inits,
                x: None,
                y: None,
                width: None,
                height: None,
            }],
            ..FlowDefinition::default()
        };
        let toml = flow.to_toml();
        assert!(toml.contains("input.start = { once = 42 }"));
    }

    #[test]
    fn flow_roundtrip() {
        let flow = FlowDefinition {
            name: Name::from("roundtrip"),
            description: "Test roundtrip".into(),
            inputs: vec![IO::new_named(
                vec![DataType::from("string")],
                Route::default(),
                "input0",
            )],
            process_refs: vec![ProcessReference {
                alias: Name::from("func"),
                source: "func.toml".into(),
                initializations: BTreeMap::new(),
                x: Some(100.0),
                y: Some(200.0),
                width: Some(180.0),
                height: Some(120.0),
            }],
            connections: vec![Connection::new("input/input0", "func")],
            ..FlowDefinition::default()
        };
        let toml = flow.to_toml();

        let url = url::Url::parse("file:///fake.toml").expect("valid url");
        let deserializer =
            crate::deserializers::deserializer::get::<FlowDefinition>(&url).expect("deserializer");
        let parsed = deserializer
            .deserialize(&toml, Some(&url))
            .expect("roundtrip parse failed");
        assert_eq!(parsed.name, "roundtrip");
        assert_eq!(parsed.description, "Test roundtrip");
        assert_eq!(parsed.inputs.len(), 1);
        assert_eq!(parsed.process_refs.len(), 1);
        assert_eq!(parsed.connections.len(), 1);
    }

    #[test]
    fn function_to_toml_minimal() {
        let func = FunctionDefinition {
            name: Name::from("myfunc"),
            source: "myfunc.rs".into(),
            ..FunctionDefinition::default()
        };
        let toml = func.to_toml();
        assert!(toml.contains("function = \"myfunc\""));
        assert!(toml.contains("source = \"myfunc.rs\""));
        assert!(toml.contains("type = \"rust\""));
    }

    #[test]
    fn function_to_toml_with_ports() {
        let func = FunctionDefinition {
            name: Name::from("add"),
            source: "add.rs".into(),
            inputs: vec![
                IO::new_named(vec![DataType::from("number")], Route::default(), "a"),
                IO::new_named(vec![DataType::from("number")], Route::default(), "b"),
            ],
            outputs: vec![IO::new_named(
                vec![DataType::from("number")],
                Route::default(),
                "sum",
            )],
            ..FunctionDefinition::default()
        };
        let toml = func.to_toml();
        assert!(toml.contains("name = \"a\""));
        assert!(toml.contains("name = \"b\""));
        assert!(toml.contains("name = \"sum\""));
        assert_eq!(toml.matches("[[input]]").count(), 2);
        assert_eq!(toml.matches("[[output]]").count(), 1);
    }

    #[test]
    fn function_roundtrip() {
        let func = FunctionDefinition {
            name: Name::from("roundtrip"),
            source: "roundtrip.rs".into(),
            description: "Test func".into(),
            inputs: vec![IO::new_named(
                vec![DataType::from("string")],
                Route::default(),
                "input0",
            )],
            outputs: vec![IO::new_named(
                vec![DataType::from("number")],
                Route::default(),
                "output0",
            )],
            ..FunctionDefinition::default()
        };
        let toml = func.to_toml();

        let url = url::Url::parse("file:///fake.toml").expect("valid url");
        let deserializer =
            crate::deserializers::deserializer::get::<crate::model::process::Process>(&url)
                .expect("deserializer");
        let parsed = deserializer
            .deserialize(&toml, Some(&url))
            .expect("roundtrip parse failed");
        if let crate::model::process::Process::FunctionProcess(f) = parsed {
            assert_eq!(f.name, "roundtrip");
            assert_eq!(f.source, "roundtrip.rs");
            assert_eq!(f.inputs.len(), 1);
            assert_eq!(f.outputs.len(), 1);
        } else {
            panic!("Expected FunctionProcess");
        }
    }

    #[test]
    fn escape_special_chars() {
        assert_eq!(escape_toml_string("hello"), "hello");
        assert_eq!(escape_toml_string("a\"b"), "a\\\"b");
        assert_eq!(escape_toml_string("a\\b"), "a\\\\b");
        assert_eq!(escape_toml_string("a\nb"), "a\\nb");
    }

    #[test]
    fn value_to_toml_types() {
        assert_eq!(value_to_toml(&serde_json::json!("hello")), "\"hello\"");
        assert_eq!(value_to_toml(&serde_json::json!(42)), "42");
        assert_eq!(value_to_toml(&serde_json::json!(true)), "true");
        assert_eq!(value_to_toml(&serde_json::json!(null)), "\"null\"");
        assert_eq!(value_to_toml(&serde_json::json!([1, 2])), "[1, 2]");
    }

    #[test]
    fn initializer_to_toml_once() {
        let init = InputInitializer::Once(serde_json::json!(42));
        assert_eq!(initializer_to_toml(&init), "{ once = 42 }");
    }

    #[test]
    fn initializer_to_toml_always() {
        let init = InputInitializer::Always(serde_json::json!("hello"));
        assert_eq!(initializer_to_toml(&init), "{ always = \"hello\" }");
    }
}
