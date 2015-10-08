use std::collections::BTreeMap;
use std::convert::From;
use serde::Serialize;
use serde_json::{self, Value};

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

    pub fn with_param<V: Serialize>(&mut self, key: &str, value: V) -> &mut Self {
        self.parameters.insert(key.to_owned(), serde_json::value::to_value(&value));
        self
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

    pub fn add_param<V: Serialize>(&mut self, key: &str, value: V) {
        self.parameters.insert(key.to_owned(), serde_json::value::to_value(&value));
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
        let stmt = Statement::new("match n return n");
    }

    #[test]
    fn with_param() {
        let mut statement = Statement::new("match n return n");
        statement.with_param("param1", "value1")
            .with_param("param2", 2)
            .with_param("param3", 3.0)
            .with_param("param4", [0; 4]);

        assert_eq!(statement.get_params().len(), 4);
    }

    #[test]
    fn add_param() {
        let mut statement = Statement::new("match n return n");
        statement.add_param("param1", "value1");
        statement.add_param("param2", 2);
        statement.add_param("param3", 3.0);
        statement.add_param("param4", [0; 4]);

        assert_eq!(statement.get_params().len(), 4);
    }

    #[test]
    fn remove_param() {
        let mut statement = Statement::new("match n return n");
        statement.with_param("param1", "value1")
            .with_param("param2", 2)
            .with_param("param3", 3.0)
            .with_param("param4", [0; 4]);

        statement.remove_param("param1");

        assert_eq!(statement.get_params().len(), 3);
    }
}
