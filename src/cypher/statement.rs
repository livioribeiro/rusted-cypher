use std::collections::BTreeMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{self, Value};
use serde_json::error::Error as JsonError;

/// Helper macro to simplify the creation of complex statements
///
/// Pass in the statement text as the first argument followed by the (optional) parameters, which
/// must be in the format `"param" => value` and wrapped in `{}`
///
/// # Examples
///
/// ```
/// # #[macro_use] extern crate rusted_cypher;
/// # use rusted_cypher::GraphError;
/// # fn main() { doctest().unwrap(); }
/// # fn doctest() -> Result<(), GraphError> {
/// // Without parameters
/// let statement = cypher_stmt!("MATCH n RETURN n")?;
/// // With parameters
/// let statement = cypher_stmt!("MATCH n RETURN n", {
///     "param1" => "value1",
///     "param2" => 2,
///     "param3" => 3.0
/// })?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! cypher_stmt {
    ( $s:expr ) => {{
        Ok($crate::Statement::new($s)) as Result<$crate::Statement, $crate::serde_json::error::Error>
    }};
    ( $s:expr, { $( $k:expr => $v:expr ),+ } ) => {{
        let mut stmt = $crate::Statement::new($s);
        let mut error: Option<$crate::serde_json::error::Error> = None;

        $(
            if error.is_none() {
                match stmt.add_param($k, $v) {
                    Err(e) => error = Some(e),
                    _ => {}
                }
            }
        )*

        if let Some(error) = error {
            Err($crate::error::GraphError::Serde(error))
        } else {
            Ok(stmt)
        }
    }}
}

/// Represents a statement to be sent to the server
#[derive(Clone, Debug, Serialize)]
pub struct Statement {
    statement: String,
    parameters: BTreeMap<String, Value>,
}

impl Statement  {
    pub fn new<T: Into<String>>(statement: T) -> Self {
        Statement {
            statement: statement.into(),
            parameters: BTreeMap::new(),
        }
    }

    /// Returns the statement text
    pub fn statement(&self) -> &str {
        &self.statement
    }

    /// Adds parameter to the `Statement`
    ///
    /// The parameter value is serialized into a `Value`. Since the serialization can fail, the
    /// method returns a `Result`
    pub fn add_param<K, V>(&mut self, key: K, value: V) -> Result<(), JsonError>
        where K: Into<String>, V: Serialize + Copy
    {
        self.parameters.insert(key.into(), serde_json::value::to_value(&value)?);
        Ok(())
    }

    /// Adds parameter in builder style
    ///
    /// This method consumes `self` and returns it with the parameter added, so the binding does
    /// not need to be mutable
    ///
    /// # Examples
    ///
    /// ```
    /// # use rusted_cypher::{Statement, GraphError};
    /// # fn main() { doctest().unwrap(); }
    /// # fn doctest() -> Result<(), GraphError> {
    /// let statement = Statement::new("MATCH n RETURN n")
    ///     .with_param("param1", "value1")?
    ///     .with_param("param2", 2)?
    ///     .with_param("param3", 3.0)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_param<K, V>(mut self, key: K, value: V) -> Result<Self, JsonError>
        where K: Into<String>, V: Serialize + Copy
    {
        self.add_param(key, value)?;
        Ok(self)
    }

    /// Gets the value of the parameter
    ///
    /// Returns `None` if there is no parameter with the given name or `Some(serde_json::error::Error)``
    /// if the parameter cannot be converted back from `serde_json::value::Value`
    pub fn param<T: DeserializeOwned>(&self, key: &str) -> Option<Result<T, serde_json::error::Error>> {
        self.parameters.get(key).map(|v| serde_json::value::from_value(v.clone()))
    }

    /// Gets a reference to the underlying parameters `BTreeMap`
    pub fn parameters(&self) -> &BTreeMap<String, Value> {
        &self.parameters
    }

    /// Sets the parameters `BTreeMap`, overriding current values
    pub fn set_parameters<T: Serialize>(&mut self, params: &BTreeMap<String, T>)
        -> Result<(), JsonError>
    {
        let mut parameters = BTreeMap::new();
        for (k, v) in params {
            parameters.insert(k.to_owned(), serde_json::value::to_value(v)?);
        }

        self.parameters = parameters;

        Ok(())
    }

    /// Removes parameter from the statment
    ///
    /// Trying to remove a non-existent parameter has no effect
    pub fn remove_param(&mut self, key: &str) {
        self.parameters.remove(key);
    }
}

impl<T: Into<String>> From<T> for Statement {
    fn from(stmt: T) -> Self {
        Statement::new(stmt)
    }
}

// impl<'a> From<&'a str> for Statement {
//     fn from(stmt: &str) -> Self {
//         Statement::new(stmt)
//     }
// }

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
            .with_param("param1", "value1").unwrap()
            .with_param("param2", 2).unwrap()
            .with_param("param3", 3.0).unwrap()
            .with_param("param4", [0; 4]).unwrap();

        assert_eq!(statement.parameters().len(), 4);
    }

    #[test]
    fn add_param() {
        let mut statement = Statement::new("MATCH n RETURN n");
        statement.add_param("param1", "value1").unwrap();
        statement.add_param("param2", 2).unwrap();
        statement.add_param("param3", 3.0).unwrap();
        statement.add_param("param4", [0; 4]).unwrap();

        assert_eq!(statement.parameters().len(), 4);
    }

    #[test]
    fn remove_param() {
        let mut statement = Statement::new("MATCH n RETURN n")
            .with_param("param1", "value1").unwrap()
            .with_param("param2", 2).unwrap()
            .with_param("param3", 3.0).unwrap()
            .with_param("param4", [0; 4]).unwrap();

        statement.remove_param("param1");

        assert_eq!(statement.parameters().len(), 3);
    }

    #[test]
    #[allow(unused_variables)]
    fn macro_without_params() {
        let stmt = cypher_stmt!("MATCH n RETURN n").unwrap();
    }

    #[test]
    fn macro_single_param() {
        let statement1 = cypher_stmt!("MATCH n RETURN n", {
            "name" => "test"
        }).unwrap();

        let param = 1;
        let statement2 = cypher_stmt!("MATCH n RETURN n", {
            "value" => param
        }).unwrap();

        assert_eq!("test", statement1.param::<String>("name").unwrap().unwrap());
        assert_eq!(param, statement2.param::<i32>("value").unwrap().unwrap());
    }

    #[test]
    fn macro_multiple_params() {
        let param = 3f32;
        let statement = cypher_stmt!("MATCH n RETURN n", {
            "param1" => "one",
            "param2" => 2,
            "param3" => param
        }).unwrap();

        assert_eq!("one", statement.param::<String>("param1").unwrap().unwrap());
        assert_eq!(2, statement.param::<i32>("param2").unwrap().unwrap());
        assert_eq!(param, statement.param::<f32>("param3").unwrap().unwrap());
    }
}
