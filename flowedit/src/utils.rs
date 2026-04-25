//! Shared utility functions used across the flow editor.

use iced::widget::canvas;
use iced::{Point, Size};

use flowcore::model::connection::Connection;
use flowcore::model::io::IO;
use flowcore::model::name::HasName;
use flowcore::model::route::Route;

use crate::node_layout::NodeLayout;

/// Generate a unique IO name with the given prefix that doesn't collide with existing names.
pub(crate) fn next_unique_io_name(prefix: &str, existing: &[IO]) -> String {
    let mut n = existing.len();
    loop {
        let candidate = format!("{prefix}{n}");
        if !existing.iter().any(|io| io.name() == &candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Derive a short display name from a source URL.
/// e.g., `"lib://flowstdlib/math/sequence"` → `"sequence"`
/// e.g., `"context://stdio/stdout"` → `"stdout"`
pub(crate) fn derive_short_name(source: &str) -> String {
    source.rsplit('/').next().unwrap_or(source).to_string()
}

/// Split a route string like "sequence/number" into ("sequence", "number")
/// or "add1" into ("add1", "")
pub(crate) fn split_route(route: &str) -> (String, String) {
    let route = route.trim_start_matches('/');
    if let Some(pos) = route.find('/') {
        (route[..pos].to_string(), route[pos + 1..].to_string())
    } else {
        (route.to_string(), String::new())
    }
}

/// Check whether a Connection references a node by alias in its from or to routes.
pub(crate) fn connection_references_node(conn: &Connection, alias: &str) -> bool {
    let (from_node, _) = split_route(conn.from().as_ref());
    if from_node == alias {
        return true;
    }
    for to_route in conn.to() {
        let (to_node, _) = split_route(to_route.as_ref());
        if to_node == alias {
            return true;
        }
    }
    false
}

/// Format a [`serde_json::Value`] for compact display
pub(crate) fn format_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => {
            serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\""))
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(a) => {
            if a.len() <= 3 {
                format!(
                    "[{}]",
                    a.iter().map(format_value).collect::<Vec<_>>().join(",")
                )
            } else {
                format!("[{}...]", a.len())
            }
        }
        serde_json::Value::Object(_) => "{...}".to_string(),
    }
}

/// Truncate a source string to fit within the node, adding an ellipsis if needed.
pub(crate) fn truncate_source(source: &str, max_len: usize) -> String {
    if source.len() <= max_len {
        source.to_string()
    } else {
        let end = source
            .char_indices()
            .nth(max_len.saturating_sub(3))
            .map_or(source.len(), |(i, _)| i);
        let mut truncated = source.get(..end).unwrap_or(source).to_string();
        truncated.push_str("...");
        truncated
    }
}

/// Extract the base port name, stripping any trailing array index.
/// Uses flowcore's Route to detect array selectors properly.
pub(crate) fn base_port_name(port: &str) -> &str {
    if Route::from(port).is_array_selector() {
        port.rsplit_once('/').map_or(port, |(base, _)| base)
    } else {
        port
    }
}

/// Build a rounded rectangle path using quadratic bezier curves at corners.
pub(crate) fn rounded_rect(
    builder: &mut canvas::path::Builder,
    top_left: Point,
    size: Size,
    radius: f32,
) {
    let cr = radius.min(size.width / 2.0).min(size.height / 2.0);
    let left = top_left.x;
    let top = top_left.y;
    let width = size.width;
    let height = size.height;

    builder.move_to(Point::new(left + cr, top));
    builder.line_to(Point::new(left + width - cr, top));
    builder.quadratic_curve_to(
        Point::new(left + width, top),
        Point::new(left + width, top + cr),
    );
    builder.line_to(Point::new(left + width, top + height - cr));
    builder.quadratic_curve_to(
        Point::new(left + width, top + height),
        Point::new(left + width - cr, top + height),
    );
    builder.line_to(Point::new(left + cr, top + height));
    builder.quadratic_curve_to(
        Point::new(left, top + height),
        Point::new(left, top + height - cr),
    );
    builder.line_to(Point::new(left, top + cr));
    builder.quadratic_curve_to(Point::new(left, top), Point::new(left + cr, top));
    builder.close();
}

/// Check if the types of two ports are compatible for a connection.
///
/// Returns true if:
/// - Either port has no type info (unknown types are assumed compatible)
/// - At least one type from the source port matches a type on the destination port
pub(crate) fn check_port_type_compatibility(
    source_node: Option<&NodeLayout>,
    source_port: &str,
    source_is_output: bool,
    target_node: &NodeLayout,
    target_port: &str,
    target_is_output: bool,
) -> bool {
    let source_types = source_node.and_then(|n| {
        let ports = if source_is_output {
            n.outputs()
        } else {
            n.inputs()
        };
        ports.iter().find(|p| p.name() == source_port)
    });

    let target_types = {
        let ports = if target_is_output {
            target_node.outputs()
        } else {
            target_node.inputs()
        };
        ports.iter().find(|p| p.name() == target_port)
    };

    match (source_types, target_types) {
        (Some(src), Some(tgt)) => {
            log::info!(
                "Type check: src port '{}' types {:?} → tgt port '{}' types {:?}",
                src.name(),
                src.datatypes(),
                tgt.name(),
                tgt.datatypes()
            );
            let src_untyped = src.datatypes().is_empty()
                || src.datatypes().iter().all(|d| d.to_string().is_empty());
            let tgt_untyped = tgt.datatypes().is_empty()
                || tgt.datatypes().iter().all(|d| d.to_string().is_empty());
            if src_untyped || tgt_untyped {
                return true;
            }
            src.datatypes()
                .iter()
                .any(|st| tgt.datatypes().iter().any(|tt| st == tt))
        }
        // Unknown port or no type info — allow
        (src, tgt) => {
            log::info!(
                "Type check: src={}, tgt={} — allowing (unknown port)",
                src.is_some(),
                tgt.is_some()
            );
            true
        }
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod test {
    use super::*;
    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::io::IO;
    use flowcore::model::process::Process;
    use flowcore::model::process_reference::ProcessReference;
    use flowcore::model::route::Route;

    fn test_node(
        alias: &str,
        source: &str,
        process: Option<Process>,
    ) -> (ProcessReference, Option<Process>) {
        (
            ProcessReference {
                alias: alias.into(),
                source: source.into(),
                initializations: std::collections::BTreeMap::new(),
                x: Some(100.0),
                y: Some(100.0),
                width: Some(180.0),
                height: Some(120.0),
            },
            process,
        )
    }

    fn as_layout(data: &(ProcessReference, Option<Process>)) -> NodeLayout<'_> {
        NodeLayout {
            process_ref: &data.0,
            process: data.1.as_ref(),
        }
    }

    fn function_with_io(inputs: Vec<IO>, outputs: Vec<IO>) -> FunctionDefinition {
        let mut f = FunctionDefinition::default();
        f.inputs = inputs;
        f.outputs = outputs;
        f
    }

    #[test]
    fn split_route_with_port() {
        let (node, port) = split_route("sequence/number");
        assert_eq!(node, "sequence");
        assert_eq!(port, "number");
    }

    #[test]
    fn split_route_no_port() {
        let (node, port) = split_route("add1");
        assert_eq!(node, "add1");
        assert_eq!(port, "");
    }

    #[test]
    fn split_route_leading_slash() {
        let (node, port) = split_route("/sequence/number");
        assert_eq!(node, "sequence");
        assert_eq!(port, "number");
    }

    #[test]
    fn derive_short_name_lib() {
        assert_eq!(
            derive_short_name("lib://flowstdlib/math/sequence"),
            "sequence"
        );
    }

    #[test]
    fn derive_short_name_context() {
        assert_eq!(derive_short_name("context://stdio/stdout"), "stdout");
    }

    #[test]
    fn derive_short_name_simple() {
        assert_eq!(derive_short_name("add"), "add");
    }

    #[test]
    fn format_value_string() {
        assert_eq!(format_value(&serde_json::json!("hello")), "\"hello\"");
    }

    #[test]
    fn format_value_number() {
        assert_eq!(format_value(&serde_json::json!(42)), "42");
    }

    #[test]
    fn format_value_bool() {
        assert_eq!(format_value(&serde_json::json!(true)), "true");
    }

    #[test]
    fn format_value_null() {
        assert_eq!(format_value(&serde_json::json!(null)), "null");
    }

    #[test]
    fn format_value_small_array() {
        assert_eq!(format_value(&serde_json::json!([1, 2, 3])), "[1,2,3]");
    }

    #[test]
    fn format_value_large_array() {
        assert_eq!(format_value(&serde_json::json!([1, 2, 3, 4])), "[4...]");
    }

    #[test]
    fn format_value_object() {
        assert_eq!(format_value(&serde_json::json!({"a": 1})), "{...}");
    }

    #[test]
    fn truncate_source_short() {
        assert_eq!(truncate_source("short", 10), "short");
    }

    #[test]
    fn truncate_source_long() {
        let result = truncate_source("this is a very long source string", 15);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 15);
    }

    #[test]
    fn truncate_source_under_limit() {
        assert_eq!(truncate_source("short", 22), "short");
    }

    #[test]
    fn truncate_source_with_ellipsis() {
        let long = "lib://flowstdlib/math/very_long_function_name";
        let result = truncate_source(long, 22);
        assert!(result.len() <= 25); // with ellipsis
        assert!(result.contains("..."));
    }

    #[test]
    fn base_port_name_simple() {
        assert_eq!(base_port_name("string"), "string");
    }

    #[test]
    fn base_port_name_with_array_index() {
        assert_eq!(base_port_name("string/1"), "string");
    }

    #[test]
    fn base_port_name_with_deep_array_index() {
        assert_eq!(base_port_name("json/3"), "json");
    }

    #[test]
    fn base_port_name_no_index() {
        assert_eq!(base_port_name("array/number"), "array/number");
    }

    #[test]
    fn base_port_name_empty() {
        assert_eq!(base_port_name(""), "");
    }

    #[test]
    fn connection_references_node_check() {
        let conn = Connection::new("a/out", "b/in");
        assert!(connection_references_node(&conn, "a"));
        assert!(connection_references_node(&conn, "b"));
        assert!(!connection_references_node(&conn, "c"));
    }

    #[test]
    fn check_type_compat_same_type() {
        let data = [
            test_node(
                "a",
                "",
                Some(Process::FunctionProcess(function_with_io(
                    vec![],
                    vec![IO::new_named(
                        vec!["number".into()],
                        Route::default(),
                        "out",
                    )],
                ))),
            ),
            test_node(
                "b",
                "",
                Some(Process::FunctionProcess(function_with_io(
                    vec![IO::new_named(vec!["number".into()], Route::default(), "in")],
                    vec![],
                ))),
            ),
        ];
        let nodes: Vec<_> = data.iter().map(as_layout).collect();
        assert!(check_port_type_compatibility(
            Some(&nodes[0]),
            "out",
            true,
            &nodes[1],
            "in",
            false
        ));
    }

    #[test]
    fn check_type_compat_different_type() {
        let data = [
            test_node(
                "a",
                "",
                Some(Process::FunctionProcess(function_with_io(
                    vec![],
                    vec![IO::new_named(
                        vec!["number".into()],
                        Route::default(),
                        "out",
                    )],
                ))),
            ),
            test_node(
                "b",
                "",
                Some(Process::FunctionProcess(function_with_io(
                    vec![IO::new_named(vec!["string".into()], Route::default(), "in")],
                    vec![],
                ))),
            ),
        ];
        let nodes: Vec<_> = data.iter().map(as_layout).collect();
        assert!(!check_port_type_compatibility(
            Some(&nodes[0]),
            "out",
            true,
            &nodes[1],
            "in",
            false
        ));
    }

    #[test]
    fn check_type_compat_untyped_allows_any() {
        let data = [
            test_node(
                "a",
                "",
                Some(Process::FunctionProcess(function_with_io(
                    vec![],
                    vec![IO::new_named(vec![], Route::default(), "out")],
                ))),
            ),
            test_node(
                "b",
                "",
                Some(Process::FunctionProcess(function_with_io(
                    vec![IO::new_named(vec!["string".into()], Route::default(), "in")],
                    vec![],
                ))),
            ),
        ];
        let nodes: Vec<_> = data.iter().map(as_layout).collect();
        assert!(check_port_type_compatibility(
            Some(&nodes[0]),
            "out",
            true,
            &nodes[1],
            "in",
            false
        ));
    }
}
