use std::convert::From;

#[cfg(not(feature = "rustc-serialize"))]
use serde::Deserialize;
use serde_json;
use serde_json::value::Value;

#[cfg(feature = "rustc-serialize")]
use rustc_serialize::Decodable;
#[cfg(feature = "rustc-serialize")]
use rustc_serialize::json as rustc_json;

use ::error::{GraphError, Neo4jError};

pub trait ResultTrait {
    fn results(&self) -> &Vec<CypherResult>;
    fn errors(&self) -> &Vec<Neo4jError>;
}

#[derive(Debug, Deserialize)]
pub struct QueryResult {
    pub results: Vec<CypherResult>,
    pub errors: Vec<Neo4jError>,
}

impl ResultTrait for QueryResult {
    fn results(&self) -> &Vec<CypherResult> {
        &self.results
    }

    fn errors(&self) -> &Vec<Neo4jError> {
        &self.errors
    }
}

/// Holds a single row of the result of a cypher query
#[derive(Clone, Debug, Deserialize)]
pub struct RowResult {
    row: Vec<Value>,
}

/// Holds the result of a cypher query
#[derive(Clone, Debug, Deserialize)]
pub struct CypherResult {
    pub columns: Vec<String>,
    pub data: Vec<RowResult>,
}

impl CypherResult {
    /// Returns an iterator over the rows of the result
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

    /// Gets the value of a column by its name
    ///
    /// The column name must be exactly as it was named in the query. For example, if your query
    /// was `"MATCH (n:PERSON) RETURN n.name, n.gender"`, then you should use `row.get("n.name")`
    /// and `row.get("n.gender")` since this is what the neo4j api produces.
    ///
    /// If the column does not exist in the row, an `Err` is returned with the message
    /// `"No such column: {column_name}"`.
    #[cfg(not(feature = "rustc-serialize"))]
    pub fn get<T: Deserialize>(&self, column: &str) -> Result<T, GraphError> {
        match self.columns.iter().position(|c| c == column) {
            Some(index) => self.get_n(index),
            None => Err(GraphError::Statement(format!("No such column: {}", &column))),
        }
    }

    #[cfg(feature = "rustc-serialize")]
    pub fn get<T: Decodable>(&self, column: &str) -> Result<T, GraphError> {
        match self.columns.iter().position(|c| c == column) {
            Some(index) => self.get_n(index),
            None => Err(GraphError::Statement(format!("No such column: {}", &column))),
        }
    }

    /// Gets the value of a column by order
    ///
    /// Column number is 0 based, so the first column is 0, the second is 1 and so on.
    ///
    /// If the column number is not within the columns length, and `Err` is returned with the
    /// message `"No such column at index {column_number}"`.
    #[cfg(not(feature = "rustc-serialize"))]
    pub fn get_n<T: Deserialize>(&self, column: usize) -> Result<T, GraphError> {
        let column_data = match self.data.get(column) {
            Some(c) => c.clone(),
            None => return Err(GraphError::Statement(format!("No column at index {}", column))),
        };

        serde_json::value::from_value::<T>(column_data).map_err(From::from)
    }

    #[cfg(feature = "rustc-serialize")]
    pub fn get_n<T: Decodable>(&self, column: usize) -> Result<T, GraphError> {
        let column_data = match self.data.get(column) {
            Some(c) => c.clone(),
            None => return Err(GraphError::Statement(format!("No column at index {}", column))),
        };

        let between = try!(serde_json::to_string(&column_data));
        rustc_json::decode(&between).map_err(From::from)
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

    #[test]
    #[should_panic(expected = "No such column")]
    fn no_column_name_in_row() {
        let result = make_result();
        let rows: Vec<Row> = result.rows().collect();
        let ref row = rows[0];
        row.get::<String>("nonexistent").unwrap();
    }

    #[test]
    #[should_panic(expected = "No column at index")]
    fn no_column_index_in_row() {
        let result = make_result();
        let rows: Vec<Row> = result.rows().collect();
        let ref row = rows[0];
        row.get_n::<String>(99).unwrap();
    }
}
