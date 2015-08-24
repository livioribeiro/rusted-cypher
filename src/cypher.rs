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
//! # fn main() {
//! # use std::collections::BTreeMap;
//! # use std::rc::Rc;
//! # use hyper::{Client, Url};
//! # use hyper::header::{Authorization, Basic, ContentType, Headers};
//! # use rusted_cypher::cypher::Cypher;
//! let url = Url::parse("http://localhost:7474/db/data/transaction").unwrap();
//! let client = Rc::new(Client::new());
//! let mut headers = Headers::new();
//! headers.set(Authorization(
//!     Basic {
//!         username: "neo4j".to_owned(),
//!         password: Some("neo4j".to_owned()),
//!     }
//! ));
//! headers.set(ContentType::json());
//! let headers = Rc::new(headers);
//! let cypher = Cypher::new(url, client, headers);
//! let mut query = cypher.query();
//! query.add_simple_statement("match n return n");
//! let result = query.send().unwrap();
//! for row in result.iter() {
//!     println!("{:?}", row);
//! }
//! # }
//! ```

use std::collections::BTreeMap;
use std::error::Error;
use std::rc::Rc;
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
    client: Rc<Client>,
    headers: Rc<Headers>,
}

impl Cypher {
    /// Creates a new Cypher
    ///
    /// Its arguments are the cypher transaction endpoint, a hyper client and the HTTP headers
    /// containing HTTP Basic Authentication, if needed.
    pub fn new(endpoint: Url, client: Rc<Client>, headers: Rc<Headers>) -> Self {
        Cypher {
            endpoint: endpoint,
            client: client,
            headers: headers,
        }
    }

    fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    /// Creates a new `CypherQuery`
    pub fn query(&self) -> CypherQuery {
        CypherQuery {
            statements: Vec::new(),
            cypher: &self,
        }
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

    pub fn send(self) -> Result<Vec<CypherResult>, Box<Error>> {
        let client = self.cypher.client.clone();
        let headers = self.cypher.headers.clone();

        let mut json = BTreeMap::new();
        json.insert("statements", self.statements);
        let json = try!(serde_json::to_string(&json));

        let cypher_commit = format!("{}/{}", self.cypher.endpoint(), "commit");
        let req = client.post(&cypher_commit)
            .headers((*headers).to_owned())
            .body(&json);

        let mut res = try!(req.send());

        let result: Value = try!(serde_json::de::from_reader(&mut res));
        match serde_json::value::from_value::<QueryResult>(result) {
            Ok(result) => {
                if result.errors.len() > 0 {
                    return Err(Box::new(GraphError::new_neo4j_error(result.errors)))
                }

                return Ok(result.results);
            }
            Err(e) => return Err(Box::new(e))
        }
    }
}

#[derive(Serialize)]
pub struct Statement {
    statement: String,
    parameters: Value,
}

impl Statement {
    pub fn new<T: Serialize>(statement: &str, parameters: T) -> Self {
        Statement {
            statement: statement.to_owned(),
            parameters: serde_json::value::to_value(&parameters),
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
pub fn get_cypher() -> Cypher {
    use hyper::header::{Authorization, Basic, ContentType, Headers};

    let cypher_endpoint = Url::parse("http://localhost:7474/db/data/transaction").unwrap();
    let client = Rc::new(Client::new());

    let mut headers = Headers::new();
    headers.set(Authorization(
        Basic {
            username: "neo4j".to_owned(),
            password: Some("neo4j".to_owned()),
        }
    ));
    headers.set(ContentType::json());
    let headers = Rc::new(headers);

    Cypher::new(cypher_endpoint, client, headers)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "create (n {name: 'test_iter', lastname: 'LastName'}), (m {name: 'test_iter', lastname: 'LastName'})");

        query.send().unwrap();

        let mut query = cypher.query();
        query.add_simple_statement("match n where n.name = 'test_iter' return n");

        let result = query.send().unwrap();

        assert_eq!(result[0].data.len(), 2);

        let result = result.get(0).unwrap();
        for row in result.iter() {
            assert!(row[0].find("name").is_some());
            assert!(row[0].find("lastname").is_some());
            assert_eq!(row[0].find("name").unwrap().as_string().unwrap(), "test_iter");
            assert_eq!(row[0].find("lastname").unwrap().as_string().unwrap(), "LastName");
        }

        let mut query = cypher.query();
        query.add_simple_statement("match n where n.name = 'test_iter' delete n");
        query.send().unwrap();
    }
}
