use model::connection::Connection;
use generator::code_gen::CodeGenTables;

pub fn collapse_connections(complete_table: &Vec<Connection>) -> Vec<Connection> {
    let mut collapsed_table: Vec<Connection> = Vec::new();

    for left in complete_table {
        if left.ends_at_flow {
            for ref right in complete_table {
                if left.to_route == right.from_route {
                    // They are connected - modify first to go to destination of second
                    let mut joined_connection = left.clone();
                    joined_connection.to_route = format!("{}", right.to_route);
                    joined_connection.ends_at_flow = right.ends_at_flow;
                    collapsed_table.push(joined_connection);
                    //                      connection_table.drop(right)
                }
            }
        } else {
            collapsed_table.push(left.clone());
        }
    }

    // Build the final connection table, leaving out the ones starting or ending at flow boundaries
    let mut final_table: Vec<Connection> = Vec::new();
    for connection in collapsed_table {
        if !connection.starts_at_flow && !connection.ends_at_flow {
            final_table.push(connection.clone());
        }
    }

    final_table
}

#[cfg(test)]
mod test {
    use model::connection::Connection;
    use super::collapse_connections;

    #[test]
    fn collapses_a_connection() {
        let left_side = Connection {
            name: Some("left".to_string()),
            from: "point a".to_string(),
            from_route: "/f1/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: false,
            to: "point b".to_string(),
            to_route: "/f2/a".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: true
        };

        let right_side = Connection {
            name: Some("right".to_string()),
            from: "point b".to_string(),
            from_route: "/f2/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: true,
            to: "point c".to_string(),
            to_route: "/f3/a".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: false
        };

        let connections = &vec!(left_side, right_side);

        let collapsed = collapse_connections(connections);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].from_route, "/f1/a".to_string());
        assert_eq!(collapsed[0].to_route, "/f3/a".to_string());
    }

    #[test]
    fn doesnt_collapse_a_non_connection() {
        let one = Connection {
            name: Some("left".to_string()),
            from: "point a".to_string(),
            from_route: "/f1/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: false,
            to: "point b".to_string(),
            to_route: "/f2/a".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: false
        };

        let other = Connection {
            name: Some("right".to_string()),
            from: "point b".to_string(),
            from_route: "/f3/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: false,
            to: "point c".to_string(),
            to_route: "/f4/a".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: false
        };

        let connections = &vec!(one, other);
        let collapsed = collapse_connections(connections);
        assert_eq!(collapsed.len(), 2);
    }
}

/*
    Drop the following combinations, with warnings:
    - values that don't have connections from them.
    - values that have only outputs and are not initialized.
    - functions that don't have connections from at least one output.
    - functions that don't have connections to all their inputs.
*/
// TODO implement this
pub fn prune_tables(_tables: &CodeGenTables) {}