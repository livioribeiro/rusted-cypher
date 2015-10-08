use serde::Deserialize;
use serde_json;
use serde_json::value::Value;

use ::error::GraphError;

#[derive(Debug, Deserialize)]
pub struct RowResult {
    row: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub struct CypherResult {
    pub columns: Vec<String>,
    pub data: Vec<RowResult>,
}

impl CypherResult {
    pub fn rows(&self) -> Rows {
        Rows::new(&self.columns, &self.data)
    }
}

pub struct Rows<'a> {
    current_index: usize,
    columns: &'a Vec<String>,
    data: &'a Vec<RowResult>,
}

impl<'a> Rows<'a> {
    pub fn new(columns: &'a Vec<String>, data: &'a Vec<RowResult>) -> Self {
        Rows {
            current_index: 0,
            columns: columns,
            data: data,
        }
    }
}

pub struct Row<'a> {
    columns: &'a Vec<String>,
    data: &'a Vec<Value>,
}

impl<'a> Row<'a> {
    pub fn new(columns: &'a Vec<String>, data: &'a Vec<Value>) -> Self {
        Row {
            columns: columns,
            data: data,
        }
    }

    pub fn get<T: Deserialize>(&self, column: &str) -> Result<T, GraphError> {
        match self.columns.iter().position(|c| c == column) {
            Some(index) => self.get_n(index),
            None => Err(GraphError::new("No such column")),
        }
    }

    pub fn get_n<T:Deserialize>(&self, column: usize) -> Result<T, GraphError> {
        let column_data = try!(serde_json::value::from_value::<T>(self.data[column].clone()));
        Ok(column_data)
    }
}

impl<'a> Iterator for Rows<'a> {
    type Item = Row<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.data.get(self.current_index).map(|data| {
            self.current_index += 1;
            Row::new(self.columns.as_ref(), data.row.as_ref())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use serde_json::value as json_value;
    use super::*;

    #[derive(Clone, Serialize)]
    struct Person {
        name: String,
        lastname: String,
    }

    fn make_result() -> CypherResult {
        let node = Person {
            name: "Test".to_owned(),
            lastname: "Result".to_owned(),
        };

        let node = json_value::to_value(&node);
        let row_data = vec![node];

        let row1 = RowResult { row: row_data.clone() };
        let row2 = RowResult { row: row_data.clone() };

        let data = vec![row1, row2];
        let columns = vec!["node".to_owned()];

        CypherResult {
            columns: columns,
            data: data,
        }
    }

    #[test]
    fn rows() {
        let result = make_result();
        for row in result.rows() {
            let row = row.get::<BTreeMap<String, String>>("node");
            assert!(row.is_ok());

            let row = row.unwrap();
            assert_eq!(row.get("name").unwrap(), "Test");
            assert_eq!(row.get("lastname").unwrap(), "Result");
        }
    }
}
