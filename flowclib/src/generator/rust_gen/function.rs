use model::function::Function;

pub fn to_code(function: &Function) -> String {
    let mut code = format!("Function::new(\"{}\".to_string(), ", function.name);
    match &function.inputs {
        &None => code.push_str(&format!("{}, ", 0)),
        &Some(ref inputs) => code.push_str(&format!("{}, ", inputs.len()))
    }
    code.push_str(&format!("{}, Box::new({}{{}}),", function.id, function.name));
    // Add the vector of tuples of elements and their inputs it's connected to
    code.push_str(" vec!(");
    for ref route in &function.output_routes {
        code.push_str(&format!("({},{}),", route.0, route.1));
    }
    code.push_str(")");

    code.push_str(")");

    code
}

#[cfg(test)]
mod test {
    use model::function::Function;
    use url::Url;
    use super::to_code;

    #[test]
    fn function_to_code() {
        let function = Function {
            name: "Stdout".to_string(),
            inputs: Some(vec!()),
            outputs: None,
            source_url: Url::parse("file:///fake/file").unwrap(),
            route: "/flow0/stdout".to_string(),
            lib_reference: None,
            output_routes: vec!(),
            id: 0,
        };

        let code = to_code(&function);
        assert_eq!(code, "Function::new(\"Stdout\".to_string(), 0, 0, Box::new(Stdout{}), vec!())")
    }
}