//! Provides structs used to interact with the cypher transaction endpoint
//!
//! The types declared in this module, save for `Statement`, don't need to be instantiated
//! directly, since they can be obtained from the `GraphClient`
//!
//! # Examples
//!
//! ```
//! # extern crate hyper;
//! # extern crate rusted_cypher;
//! # use std::collections::BTreeMap;
//! # use hyper::Url;
//! # use hyper::header::{Authorization, Basic, ContentType, Headers};
//! # use rusted_cypher::cypher::Cypher;
//! # fn main() {
//! # let url = Url::parse("http://localhost:7474/db/data/transaction").unwrap();
//! #
//! # let mut headers = Headers::new();
//! # headers.set(Authorization(
//! #     Basic {
//! #         username: "neo4j".to_owned(),
//! #         password: Some("neo4j".to_owned()),
//! #     }
//! # ));
//! #
//! # headers.set(ContentType::json());
//!
//! let cypher = Cypher::new(url, headers);
//!
//! let mut query = cypher.query();
//! query.add_simple_statement("match n return n");
//!
//! let result = query.send().unwrap();
//!
//! for row in result.iter() {
//!     println!("{:?}", row);
//! }
//! # }
//! ```

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::error::Error;
use hyper::{Client, Url};
use hyper::header::Headers;
use serde::Serialize;
use serde_json::{self, Value};

use super::error::{GraphError, Neo4jError};

/// Represents the cypher endpoint of a neo4j server
///
/// The `Cypher` struct holds information about the cypher enpoint. It is used to create the queries
/// that are sent to the server.
pub struct Cypher {
    endpoint: Url,
    client: Client,
    headers: Headers,
}

impl Cypher {
    /// Creates a new Cypher
    ///
    /// Its arguments are the cypher transaction endpoint and the HTTP headers containing HTTP
    /// Basic Authentication, if needed.
    pub fn new(endpoint: Url, headers: Headers) -> Self {
        Cypher {
            endpoint: endpoint,
            client: Client::new(),
            headers: headers,
        }
    }

    fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    fn client(&self) -> &Client {
        &self.client
    }

    /// Creates a new `CypherQuery`
    pub fn query(&self) -> CypherQuery {
        CypherQuery {
            statements: Vec::new(),
            cypher: &self,
        }
    }

    /// Executes a cypher query
    ///
    /// # Examples
    ///
    /// ```
    /// # use rusted_cypher::GraphClient;
    /// # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data").unwrap();
    /// # let cypher = graph.cypher();
    /// let result = cypher.exec("match n return n");
    /// # let result = result.unwrap();
    /// # assert_eq!(result[0].columns.len(), 1);
    /// # assert_eq!(result[0].columns[0], "n");
    /// ```
    pub fn exec(&self, statement: &str) -> Result<Vec<CypherResult>, GraphError> {
        self.exec_params(statement, &BTreeMap::<String, Value>::new())
    }

    /// Executes a cypher query with parameters
    ///
    /// ```
    /// # use std::collections::BTreeMap;
    /// # use rusted_cypher::GraphClient;
    /// # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data").unwrap();
    /// # let cypher = graph.cypher();
    /// let mut params = BTreeMap::new();
    /// params.insert("name", "Rust Language");
    /// let result = cypher.exec_params("match (n {name: {name}}) return n", &params);
    /// # let result = result.unwrap();
    /// # assert_eq!(result[0].columns.len(), 1);
    /// # assert_eq!(result[0].columns[0], "n");
    /// ```
    pub fn exec_params<K, V>(&self, statement: &str, parameters: &BTreeMap<K ,V>)
            -> Result<Vec<CypherResult>, GraphError>
            where K: Borrow<str> + Ord + Serialize, V: Serialize {

        let mut query = self.query();
        query.add_statement(Statement::new(statement, parameters));

        query.send()
    }
}

/// Represents a cypher query
///
/// A cypher query is composed by statements, each one containing the query itself and its parameters.
///
/// The query parameters must implement `Serialize` so they can be serialized into JSON in order to
/// be sent to the server
pub struct CypherQuery<'a> {
    statements: Vec<Statement>,
    cypher: &'a Cypher,
}

impl<'a> CypherQuery<'a> {
    pub fn add_simple_statement(&mut self, statement: &str) {
        self.statements.push(Statement {
            statement: statement.to_owned(),
            parameters: Value::Null,
        });
    }

    pub fn add_statement(&mut self, statement: Statement) {
        self.statements.push(statement);
    }

    pub fn set_statements(&mut self, statements: Vec<Statement>) {
        self.statements = statements;
    }

    /// Sends the query to the server
    ///
    /// The statements contained in the query are sent to the server and the results are parsed
    /// into a `Vec<CypherResult>` in order to match the response of the neo4j api. If there is an
    /// error, a `GraphError` is returned.
    pub fn send(self) -> Result<Vec<CypherResult>, GraphError> {
        let headers = self.cypher.headers.clone();

        let mut json = BTreeMap::new();
        json.insert("statements", self.statements);
        let json = try!(serde_json::to_string(&json));

        let cypher_commit = format!("{}/{}", self.cypher.endpoint(), "commit");
        let req = self.cypher.client().post(&cypher_commit)
            .headers(headers)
            .body(&json);

        let mut res = try!(req.send());

        let result: Value = try!(serde_json::de::from_reader(&mut res));
        match serde_json::value::from_value::<QueryResult>(result) {
            Ok(result) => {
                if result.errors.len() > 0 {
                    return Err(GraphError::new_neo4j_error(result.errors))
                }

                return Ok(result.results);
            }
            Err(e) => return Err(GraphError::new_error(Box::new(e)))
        }
    }
}

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

#[derive(Debug, Deserialize)]
pub struct CypherResult {
    pub columns: Vec<String>,
    pub data: Vec<Value>,
}

impl CypherResult {
    pub fn iter(&self) -> Iter {
        Iter::new(&self.data)
    }
}

pub struct Iter<'a> {
    current_index: usize,
    data: &'a Vec<Value>,
}

impl<'a> Iter<'a> {
    pub fn new(data: &'a Vec<Value>) -> Self {
        Iter {
            current_index: 0_usize,
            data: data,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Vec<Value>;
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.data.get(self.current_index);
        item.map(|i| {
            self.current_index += 1;
            i.find("row").expect("Wrong result. Missing 'row' property").as_array().unwrap()
        })
    }
}

#[derive(Debug, Deserialize)]
struct QueryResult {
    results: Vec<CypherResult>,
    errors: Vec<Neo4jError>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_cypher() -> Cypher {
        use hyper::Url;
        use hyper::header::{Authorization, Basic, ContentType, Headers};

        let cypher_endpoint = Url::parse("http://localhost:7474/db/data/transaction").unwrap();

        let mut headers = Headers::new();
        headers.set(Authorization(
            Basic {
                username: "neo4j".to_owned(),
                password: Some("neo4j".to_owned()),
            }
        ));
        headers.set(ContentType::json());

        Cypher::new(cypher_endpoint, headers)
    }

    #[test]
    fn query() {
        let cypher = get_cypher();
        let mut query = cypher.query();

        query.add_simple_statement("match n return n");

        let result = query.send().unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");
    }

    #[test]
    fn iter() {
        let cypher = get_cypher();
        let mut query = cypher.query();

        query.add_simple_statement(
            "create (n:TEST_ITER {name: 'Test', lastname: 'Iter'}), (m:TEST_ITER {name: 'Test', lastname: 'Iter'})");

        query.send().unwrap();

        let mut query = cypher.query();
        query.add_simple_statement("match (n:TEST_ITER) return n");

        let result = query.send().unwrap();

        assert_eq!(result[0].data.len(), 2);

        let result = result.get(0).unwrap();
        for row in result.iter() {
            assert!(row[0].find("name").is_some());
            assert!(row[0].find("lastname").is_some());
            assert_eq!(row[0].find("name").unwrap().as_string().unwrap(), "Test");
            assert_eq!(row[0].find("lastname").unwrap().as_string().unwrap(), "Iter");
        }

        let mut query = cypher.query();
        query.add_simple_statement("match (n:TEST_ITER) delete n");
        query.send().unwrap();
    }
}
