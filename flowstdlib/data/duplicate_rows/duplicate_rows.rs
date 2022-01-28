use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

#[derive(FlowImpl)]
/// Duplicate the rows of a matrix
#[derive(Debug)]
pub struct DuplicateRows;

impl Implementation for DuplicateRows {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut output_matrix: Vec<Value> = vec![];

        if let Some(factor) = inputs[1].as_i64() {
            if let Some(matrix) = inputs[0].as_array() {
                for row in matrix.iter() {
                    for _i in 0..factor {
                        output_matrix.push(row.clone());
                    }
                }
            }
        }

        (Some(Value::Array(output_matrix)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flowcore::Implementation;
    use serde_json::{Number, Value};
    use serde_json::json;

    #[test]
    fn duplicate_2() {
        let row0 = json!([1, 2]);
        let row1 = json!([3, 4]);
        let matrix = Value::Array(vec![row0, row1]);

        let inputs = vec![matrix, json!(2)];

        let duplicator = super::DuplicateRows {};
        let (result, _) = duplicator.run(&inputs);

        let new_matrix = result.expect("Could not get the Value from the output");
        let new_row0 = new_matrix[0].clone();
        let new_row1 = new_matrix[1].clone();
        let new_row2 = new_matrix[2].clone();
        let new_row3 = new_matrix[3].clone();

        assert_eq!(
            new_row0,
            Value::Array(vec!(
                Value::Number(Number::from(1)),
                Value::Number(Number::from(2))
            ))
        );
        assert_eq!(
            new_row1,
            Value::Array(vec!(
                Value::Number(Number::from(1)),
                Value::Number(Number::from(2))
            ))
        );
        assert_eq!(
            new_row2,
            Value::Array(vec!(
                Value::Number(Number::from(3)),
                Value::Number(Number::from(4))
            ))
        );
        assert_eq!(
            new_row3,
            Value::Array(vec!(
                Value::Number(Number::from(3)),
                Value::Number(Number::from(4))
            ))
        );
    }

    #[test]
    fn duplicate_3() {
        let row0 = json!([1, 2]);
        let row1 = json!([3, 4]);
        let matrix = Value::Array(vec![row0, row1]);

        let inputs = vec![matrix, json!(3)];

        let duplicator = super::DuplicateRows {};
        let (result, _) = duplicator.run(&inputs);

        let new_matrix = result.expect("Could not get the Value from the output");
        let new_row0 = new_matrix[0].clone();
        let new_row1 = new_matrix[1].clone();
        let new_row2 = new_matrix[2].clone();
        let new_row3 = new_matrix[3].clone();
        let new_row4 = new_matrix[4].clone();
        let new_row5 = new_matrix[5].clone();

        assert_eq!(new_row0, json!([1, 2]));
        assert_eq!(new_row1, json!([1, 2]));
        assert_eq!(new_row2, json!([1, 2]));
        assert_eq!(new_row3, json!([3, 4]));
        assert_eq!(new_row4, json!([3, 4]));
        assert_eq!(new_row5, json!([3, 4]));
    }
}
