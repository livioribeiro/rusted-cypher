use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::convert::From;
use std::str::FromStr;
use serde::Serialize;
use serde_json::{self, Value};

#[derive(Serialize)]
pub struct Statement {
    statement: String,
    parameters: Value,
}

impl Statement  {
    pub fn new<K, V>(statement: &str, parameters: &BTreeMap<K, V>) -> Self
        where K: Borrow<str> + Ord + Serialize, V: Serialize {

        Statement {
            statement: statement.to_owned(),
            parameters: serde_json::value::to_value(parameters),
        }
    }
}

impl<'a> From<&'a str> for Statement {
    fn from(val: &str) -> Self {
        Statement {
            statement: val.to_owned(),
            parameters: Value::Null,
        }
    }
}

impl<'a, 'b, K, V> From<(&'a str, &'b BTreeMap<K, V>)> for Statement
        where K: Borrow<str> + Ord + Serialize, V: Serialize {
    fn from(val: (&str, &BTreeMap<K, V>)) -> Self {
        Statement {
            statement: val.0.to_owned(),
            parameters: serde_json::value::to_value(val.1),
        }
    }
}

impl FromStr for Statement {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Statement {
            statement: s.to_owned(),
            parameters: Value::Null,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::convert::From;
    use std::collections::BTreeMap;
    use super::*;

    #[test]
    #[allow(unused_variables)]
    fn from_str() {
        let stmt = Statement::from("match n return n");
    }

    #[test]
    #[allow(unused_variables)]
    fn from_tuple() {
        let params: BTreeMap<String, String> = BTreeMap::new();
        let stmt = Statement::from(("match n return n", &params));
    }
}
