use std::collections::BTreeMap;
use std::convert::From;
use serde::{Serialize, Deserialize};
use serde_json::{self, Value};

#[macro_export]
macro_rules! cypher_stmt {
    ( $s:expr ) => { $crate::Statement::new($s) };
    ( $s:expr { $( $k:expr => $v:expr ),+ } ) => {
        $crate::Statement::new($s)
            $(.with_param($k, $v))*
    }
}

#[derive(Clone, Serialize)]
pub struct Statement {
    statement: String,
    parameters: BTreeMap<String, Value>,
}

impl Statement  {
    pub fn new(statement: &str) -> Self {
        Statement {
            statement: statement.to_owned(),
            parameters: BTreeMap::new(),
        }
    }

    pub fn with_param<V: Serialize + Copy>(mut self, key: &str, value: V) -> Self {
        self.add_param(key, value);
        self
    }

    pub fn add_param<V: Serialize + Copy>(&mut self, key: &str, value: V) {
        self.parameters.insert(key.to_owned(), serde_json::value::to_value(&value));
    }

    pub fn get_param<V: Deserialize>(&self, key: &str) -> Option<Result<V, serde_json::error::Error>> {
        self.parameters.get(key.into()).map(|v| serde_json::value::from_value(v.clone()))
    }

    pub fn get_params(&self) -> &BTreeMap<String, Value> {
        &self.parameters
    }

    pub fn set_params<V: Serialize>(&mut self, params: &BTreeMap<String, V>) {
        let mut _params = BTreeMap::new();

        for (k, v) in params.iter() {
            _params.insert(k.to_owned(), serde_json::value::to_value(&v));
        }

        self.parameters = _params;
    }

    pub fn remove_param(&mut self, key: &str) {
        self.parameters.remove(key);
    }
}

impl<'a> From<&'a str> for Statement {
    fn from(stmt: &str) -> Self {
        Statement::new(stmt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(unused_variables)]
    fn from_str() {
        let stmt = Statement::new("MATCH n RETURN n");
    }

    #[test]
    fn with_param() {
        let statement = Statement::new("MATCH n RETURN n")
            .with_param("param1", "value1")
            .with_param("param2", 2)
            .with_param("param3", 3.0)
            .with_param("param4", [0; 4]);

        assert_eq!(statement.get_params().len(), 4);
    }

    #[test]
    fn add_param() {
        let mut statement = Statement::new("MATCH n RETURN n");
        statement.add_param("param1", "value1");
        statement.add_param("param2", 2);
        statement.add_param("param3", 3.0);
        statement.add_param("param4", [0; 4]);

        assert_eq!(statement.get_params().len(), 4);
    }

    #[test]
    fn remove_param() {
        let mut statement = Statement::new("MATCH n RETURN n")
            .with_param("param1", "value1")
            .with_param("param2", 2)
            .with_param("param3", 3.0)
            .with_param("param4", [0; 4]);

        statement.remove_param("param1");

        assert_eq!(statement.get_params().len(), 3);
    }

    #[test]
    #[allow(unused_variables)]
    fn macro_without_params() {
        let stmt = cypher_stmt!("MATCH n RETURN n");
    }

    #[test]
    fn macro_single_param() {
        let statement1 = cypher_stmt!("MATCH n RETURN n" {
            "name" => "test"
        });

        let param = 1;
        let statement2 = cypher_stmt!("MATCH n RETURN n" {
            "value" => param
        });

        assert_eq!("test", statement1.get_param::<String>("name").unwrap().unwrap());
        assert_eq!(param, statement2.get_param::<i32>("value").unwrap().unwrap());
    }

    #[test]
    fn macro_multiple_params() {
        let param = 3f32;
        let statement = cypher_stmt!("MATCH n RETURN n" {
            "param1" => "one",
            "param2" => 2,
            "param3" => param
        });

        assert_eq!("one", statement.get_param::<String>("param1").unwrap().unwrap());
        assert_eq!(2, statement.get_param::<i32>("param2").unwrap().unwrap());
        assert_eq!(param, statement.get_param::<f32>("param3").unwrap().unwrap());
    }
}
