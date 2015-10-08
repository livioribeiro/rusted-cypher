use std::convert::Into;
use std::collections::BTreeMap;
use std::error::Error;
use hyper::{Client, Url};
use hyper::header::Headers;
use serde::Deserialize;
use serde_json::{self, Value};

use ::error::{GraphError, Neo4jError};
use super::transaction::Transaction;
use super::statement::Statement;

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
    /// Parameter can be anything that implements `Into<Statement>`, `&str` or or `Statement` itself
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
    pub fn exec<S: Into<Statement>>(&self, statement: S) -> Result<Vec<CypherResult>, GraphError> {
        let mut query = self.query();
        query.add_statement(statement);

        query.send()
    }

    pub fn begin_transaction(&self, statements: Vec<Statement>) -> Result<(Transaction, Vec<CypherResult>), GraphError> {
        Transaction::begin(&format!("{}", &self.endpoint), &self.headers, statements)
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
    /// Adds a statement to the query
    ///
    /// The statement can be anything that implements Into<Statement>,
    /// currently &str and (&str, &BTreeMap<String, Value>).
    /// Statement itself already implements Into<Statement>.
    pub fn add_statement<T: Into<Statement>>(&mut self, statement: T) {
        self.statements.push(statement.into());
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

#[derive(Debug, Deserialize)]
pub struct CypherResult {
    pub columns: Vec<String>,
    pub data: Vec<Value>,
}

impl CypherResult {
    pub fn iter(&self) -> Iter {
        Iter::new(&self.columns, &self.data)
    }
}

pub struct Iter<'a> {
    current_index: usize,
    columns: &'a Vec<String>,
    data: &'a Vec<Value>,
}

impl<'a> Iter<'a> {
    pub fn new(columns: &'a Vec<String>, data: &'a Vec<Value>) -> Self {
        Iter {
            current_index: 0,
            columns: columns,
            data: data,
        }
    }
}

pub struct IterItem<'a> {
    columns: &'a Vec<String>,
    data: Vec<Value>,
}

impl<'a> IterItem<'a> {
    pub fn new(columns: &'a Vec<String>, data: Vec<Value>) -> Self {
        IterItem {
            columns: columns,
            data: data,
        }
    }

    pub fn get<T: Deserialize>(&self, column: &str) -> Result<T, serde_json::error::Error> {
        match self.columns.iter().position(|c| c == column) {
            Some(index) => serde_json::value::from_value::<T>(self.data[index].clone()),
            None => panic!("No such column"),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = IterItem<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.data.get(self.current_index).map(|item| {
            self.current_index += 1;
            match item.find("row") {
                Some(row) => IterItem::new(self.columns.as_ref(), row.as_array().expect("Wrong result").to_owned()),
                None => panic!("Wrong result. Missing 'row' property"),
            }
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
    use std::collections::BTreeMap;
    use super::*;
    use ::cypher::statement::Statement;

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

        query.add_statement("match n return n");

        let result = query.send().unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");
    }

    #[test]
    fn iter() {
        let cypher = get_cypher();
        let mut query = cypher.query();

        query.add_statement(
            "create (n:TEST_ITER {name: 'Test', lastname: 'Iter'}), (m:TEST_ITER {name: 'Test', lastname: 'Iter'})");

        query.send().unwrap();

        let mut query = cypher.query();
        query.add_statement("match (n:TEST_ITER) return n as node");

        let result = query.send().unwrap();

        assert_eq!(result[0].data.len(), 2);

        let result = result.get(0).unwrap();
        for item in result.iter() {
            let item = item.get::<BTreeMap<String, String>>("node");
            assert!(item.is_ok());

            let item = item.unwrap();
            assert_eq!(item.get("name").unwrap(), "Test");
            assert_eq!(item.get("lastname").unwrap(), "Iter");
        }

        let mut query = cypher.query();
        query.add_statement("match (n:TEST_ITER) delete n");
        query.send().unwrap();
    }

    #[test]
    fn transaction() {
        let cypher = get_cypher();

        let stmt = Statement::new("create (n:CYPHER_TRANSACTION) return n");
        let (transaction, results) = cypher.begin_transaction(vec![stmt]).unwrap();

        assert_eq!(results[0].data.len(), 1);

        transaction.commit().unwrap();

        let stmt = Statement::new("match (n:CYPHER_TRANSACTION) delete n");
        let (transaction, _) = cypher.begin_transaction(vec![stmt]).unwrap();
        transaction.commit().unwrap();
    }
}
