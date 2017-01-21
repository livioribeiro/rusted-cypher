//! Provides structs used to interact with the cypher transaction endpoint
//!
//! The types declared in this module, save for `Statement`, don't need to be instantiated
//! directly, since they can be obtained from the `GraphClient`.
//!
//! # Examples
//!
//! ## Execute a single query
//! ```
//! # use rusted_cypher::GraphClient;
//! # const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";
//! let graph = GraphClient::connect(URL).unwrap();
//!
//! graph.cypher().exec("CREATE (n:CYPHER_QUERY {value: 1})").unwrap();
//! let result = graph.cypher().exec("MATCH (n:CYPHER_QUERY) RETURN n.value AS value").unwrap();
//! # assert_eq!(result.data.len(), 1);
//!
//! // Iterate over the results
//! for row in result.rows() {
//!     let value = row.get::<i32>("value").unwrap(); // or: let value: i32 = row.get("value");
//!     assert_eq!(value, 1);
//! }
//! # graph.cypher().exec("MATCH (n:CYPHER_QUERY) delete n");
//! ```
//!
//! ## Execute multiple queries
//! ```
//! # use rusted_cypher::GraphClient;
//! # const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";
//! # let graph = GraphClient::connect(URL).unwrap();
//! let mut query = graph.cypher().query()
//!     .with_statement("MATCH (n:SOME_CYPHER_QUERY) RETURN n.value as value")
//!     .with_statement("MATCH (n:OTHER_CYPHER_QUERY) RETURN n");
//!
//! let results = query.send().unwrap();
//!
//! for row in results[0].rows() {
//!     let value: i32 = row.get("value").unwrap();
//!     assert_eq!(value, 1);
//! }
//! ```
//!
//! ## Start a transaction
//! ```
//! # use rusted_cypher::GraphClient;
//! # const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";
//! # let graph = GraphClient::connect(URL).unwrap();
//! let (transaction, results) = graph.cypher().transaction()
//!     .with_statement("MATCH (n:TRANSACTION_CYPHER_QUERY) RETURN n")
//!     .begin().unwrap();
//!
//! # assert_eq!(results.len(), 1);
//! ```

pub mod transaction;
pub mod statement;
pub mod result;

pub use self::statement::Statement;
pub use self::transaction::Transaction;
pub use self::result::CypherResult;

use std::convert::Into;
use std::collections::BTreeMap;

use hyper::client::{Client, Response};
use hyper::header::Headers;
use url::Url;
use serde::Deserialize;
use serde_json::{self, Value};
use serde_json::de as json_de;
use serde_json::ser as json_ser;
use serde_json::value as json_value;

use self::result::{QueryResult, ResultTrait};
use ::error::GraphError;

fn send_query(client: &Client, endpoint: &str, headers: &Headers, statements: Vec<Statement>)
    -> Result<Response, GraphError> {

    let mut json = BTreeMap::new();
    json.insert("statements", statements);

    let json = serde_json::to_string(&json)?;

    let req = client.post(endpoint)
        .headers(headers.clone())
        .body(&json);

    debug!("Seding query:\n{}", json_ser::to_string_pretty(&json).unwrap_or(String::new()));

    req.send().map_err(From::from)
}

fn parse_response<T: Deserialize + ResultTrait>(res: &mut Response) -> Result<T, GraphError> {
    let result: Value = json_de::from_reader(res)?;

    if let Some(errors) = result.find("errors") {
        if result.find("results").is_none() {
            return Err(GraphError::Neo4j(json_value::from_value(errors.clone())?))
        }
    }

    json_value::from_value::<T>(result)
        .map_err(|e| {
            error!("Unable to parse response: {}", &e);
            From::from(e)
        })
}

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
    pub fn new(endpoint: Url, client: Client, headers: Headers) -> Self {
        Cypher {
            endpoint: endpoint,
            client: client,
            headers: headers,
        }
    }

    #[allow(dead_code)]
    fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    fn endpoint_commit(&self) -> String {
        format!("{}/{}", &self.endpoint, "commit")
    }

    fn client(&self) -> &Client {
        &self.client
    }

    fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Creates a new `CypherQuery`
    pub fn query(&self) -> CypherQuery {
        CypherQuery {
            statements: Vec::new(),
            cypher: &self,
        }
    }

    /// Executes the given `Statement`
    ///
    /// Parameter can be anything that implements `Into<Statement>`, `&str` or `Statement` itself
    pub fn exec<S: Into<Statement>>(&self, statement: S) -> Result<CypherResult, GraphError> {
        self.query()
            .with_statement(statement)
            .send()?
            .pop()
            .ok_or(GraphError::Other("No results returned from server".to_owned()))
    }

    /// Creates a new `Transaction`
    pub fn transaction(&self) -> Transaction<self::transaction::Created> {
        Transaction::new(&self.endpoint.to_string(), &self.headers)
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
    /// Adds statements in builder style
    pub fn with_statement<T: Into<Statement>>(mut self, statement: T) -> Self {
        self.add_statement(statement);
        self
    }

    pub fn add_statement<T: Into<Statement>>(&mut self, statement: T) {
        self.statements.push(statement.into());
    }

    pub fn statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn set_statements(&mut self, statements: Vec<Statement>) {
        self.statements = statements;
    }

    /// Sends the query to the server
    ///
    /// The statements contained in the query are sent to the server and the results are parsed
    /// into a `Vec<CypherResult>` in order to match the response of the neo4j api.
    pub fn send(self) -> Result<Vec<CypherResult>, GraphError> {
        let mut res = send_query(self.cypher.client(),
                   &self.cypher.endpoint_commit(),
                   self.cypher.headers(),
                   self.statements)?;

        let result: QueryResult = parse_response(&mut res)?;
        if result.errors().len() > 0 {
            return Err(GraphError::Neo4j(result.errors().clone()))
        }

        Ok(result.results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::cypher::result::Row;

    fn get_cypher() -> Cypher {
        use hyper::{Client, Url};
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

        Cypher::new(cypher_endpoint, Client::new(), headers)
    }

    #[test]
    fn query_without_params() {
        let result = get_cypher().exec("MATCH (n:TEST_CYPHER) RETURN n").unwrap();

        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.columns[0], "n");
    }

    #[test]
    fn query_with_string_param() {
        let statement = Statement::new("MATCH (n:TEST_CYPHER {name: {name}}) RETURN n")
            .with_param("name", "Neo");

        let result = get_cypher().exec(statement).unwrap();

        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.columns[0], "n");
    }

    #[test]
    fn query_with_int_param() {
        let statement = Statement::new("MATCH (n:TEST_CYPHER {value: {value}}) RETURN n")
            .with_param("value", 42);

        let result = get_cypher().exec(statement).unwrap();

        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.columns[0], "n");
    }

    #[test]
    fn query_with_complex_param() {
        #[derive(Serialize, Deserialize)]
        pub struct ComplexType {
            pub name: String,
            pub value: i32,
        }

        let cypher = get_cypher();

        let complex_param = ComplexType {
            name: "Complex".to_owned(),
            value: 42,
        };

        let statement = Statement::new("CREATE (n:TEST_CYPHER_COMPLEX_PARAM {p})")
            .with_param("p", &complex_param);

        let result = cypher.exec(statement);
        assert!(result.is_ok());

        let results = cypher.exec("MATCH (n:TEST_CYPHER_COMPLEX_PARAM) RETURN n").unwrap();
        let rows: Vec<Row> = results.rows().take(1).collect();
        let row = rows.first().unwrap();

        let complex_result: ComplexType = row.get("n").unwrap();
        assert_eq!(complex_result.name, "Complex");
        assert_eq!(complex_result.value, 42);

        cypher.exec("MATCH (n:TEST_CYPHER_COMPLEX_PARAM) DELETE n").unwrap();
    }

    #[test]
    fn query_with_multiple_params() {
        let statement = Statement::new(
            "MATCH (n:TEST_CYPHER {name: {name}}) WHERE n.value = {value} RETURN n")
            .with_param("name", "Neo")
            .with_param("value", 42);

        let result = get_cypher().exec(statement).unwrap();
        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.columns[0], "n");
    }

    #[test]
    fn multiple_queries() {
        let cypher = get_cypher();
        let statement1 = Statement::new("MATCH (n:TEST_CYPHER) RETURN n");
        let statement2 = Statement::new("MATCH (n:TEST_CYPHER) RETURN n");

        let query = cypher.query()
            .with_statement(statement1)
            .with_statement(statement2);

        let results = query.send().unwrap();
        assert_eq!(results.len(), 2);
    }
}
