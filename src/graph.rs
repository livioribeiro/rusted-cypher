//! Main interface for executing queries against a neo4j database
//!
//! # Examples
//!
//! ## Execute a single query
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # fn main() { doctest().unwrap(); }
//! # fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data")?;
//! graph.exec("CREATE (n:CYPHER_QUERY {value: 1})")?;
//!
//! let result = graph.exec("MATCH (n:CYPHER_QUERY) RETURN n.value AS value")?;
//! # assert_eq!(result.data.len(), 1);
//!
//! // Iterate over the results
//! for row in result.rows() {
//!     let value = row.get::<i32>("value")?; // or: let value: i32 = row.get("value")?;
//!     assert_eq!(value, 1);
//! }
//! # graph.exec("MATCH (n:CYPHER_QUERY) delete n")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Execute multiple queries
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # fn main() { doctest().unwrap(); }
//! # fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data")?;
//! let mut query = graph.query()
//!     .with_statement("MATCH (n:SOME_CYPHER_QUERY) RETURN n.value as value")
//!     .with_statement("MATCH (n:OTHER_CYPHER_QUERY) RETURN n");
//!
//! let results = query.send()?;
//!
//! for row in results[0].rows() {
//!     let value: i32 = row.get("value")?;
//!     assert_eq!(value, 1);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Start a transaction
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # fn main() { doctest().unwrap(); }
//! # fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data")?;
//! let (transaction, results) = graph.transaction()
//!     .with_statement("MATCH (n:TRANSACTION_CYPHER_QUERY) RETURN n")
//!     .begin()?;
//! # assert_eq!(results.len(), 1);
//! # Ok(())
//! }
//! ```

use std::collections::BTreeMap;
use std::io::Read;
use hyper::{Client, Url};
use hyper::header::{Authorization, Basic, ContentType, Headers};
use serde_json::{self, Value};
use serde_json::value as json_value;
use semver::Version;

use cypher::{Cypher, CypherQuery, CypherResult};
use cypher::transaction::{Transaction, Created as TransactionCreated};
use cypher::statement::Statement;
use error::GraphError;

#[derive(Deserialize)]
pub struct ServiceRoot {
    pub extensions: BTreeMap<String, Value>,
    pub node: String,
    pub node_index: String,
    pub relationship_index: String,
    pub extensions_info: String,
    pub relationship_types: String,
    pub batch: String,
    pub cypher: String,
    pub indexes: String,
    pub constraints: String,
    pub transaction: String,
    pub node_labels: String,
    pub neo4j_version: String,
}

fn decode_service_root<R: Read>(reader: &mut R) -> Result<ServiceRoot, GraphError> {
    let result: Value = serde_json::de::from_reader(reader)?;

    if let Some(errors) = result.get("errors") {
        if errors.as_array().map(|a| a.len()).unwrap_or(0) > 0 {
            return Err(GraphError::Neo4j(json_value::from_value(errors.clone())?))
        }
    }

    json_value::from_value(result)
        .map_err(From::from)
}

#[allow(dead_code)]
pub struct GraphClient {
    headers: Headers,
    service_root: ServiceRoot,
    neo4j_version: Version,
    cypher: Cypher,
}

impl GraphClient {
    pub fn connect<T: AsRef<str>>(endpoint: T) -> Result<Self, GraphError> {
        let endpoint = endpoint.as_ref();
        let url = Url::parse(endpoint)
            .map_err(|e| {
                error!("Unable to parse URL");
                e
            })?;

        let mut headers = Headers::new();

        url.password().map(|password| {
            headers.set(Authorization(
                Basic {
                    username: url.username().to_owned(),
                    password: Some(password.to_owned()),
                }
            ));
        });

        headers.set(ContentType::json());

        let client = Client::new();
        let mut res = client.get(endpoint)
            .headers(headers.clone())
            .send()
            .map_err(|e| {
                error!("Unable to connect to server: {}", &e);
                e
            })?;

        let service_root = decode_service_root(&mut res)?;

        let neo4j_version = Version::parse(&service_root.neo4j_version)?;
        let cypher_endpoint = Url::parse(&service_root.transaction)?;

        let cypher = Cypher::new(cypher_endpoint, client, headers.clone());

        Ok(GraphClient {
            headers: headers,
            service_root: service_root,
            neo4j_version: neo4j_version,
            cypher: cypher,
        })
    }

    /// Creates a new `CypherQuery`
    pub fn query(&self) -> CypherQuery {
        self.cypher.query()
    }

    /// Executes the given `Statement`
    ///
    /// Parameter can be anything that implements `Into<Statement>`, `Into<String>` or `Statement`
    /// itself
    pub fn exec<S: Into<Statement>>(&self, statement: S) -> Result<CypherResult, GraphError> {
        self.cypher.exec(statement)
    }

    /// Creates a new `Transaction`
    pub fn transaction(&self) -> Transaction<TransactionCreated> {
        self.cypher.transaction()
    }

    pub fn neo4j_version(&self) -> &Version {
        &self.neo4j_version
    }

    /// Returns a reference to the `Cypher` instance of the `GraphClient`
    #[deprecated(since = "1.0.0", note = "Use methods on `GraphClient` instead")]
    pub fn cypher(&self) -> &Cypher {
        &self.cypher
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";

    #[test]
    fn connect() {
        let graph = GraphClient::connect(URL);
        assert!(graph.is_ok());
        let graph = graph.unwrap();
        assert!(graph.neo4j_version().major >= 2);
    }

    #[test]
    fn query() {
        let graph = GraphClient::connect(URL).unwrap();

        let mut query = graph.query();
        query.add_statement("MATCH (n) RETURN n");

        let result = query.send().unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");
    }

    #[test]
    fn transaction() {
        let graph = GraphClient::connect(URL).unwrap();

        let (transaction, result) = graph.transaction()
            .with_statement("MATCH (n) RETURN n")
            .begin()
            .unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");

        transaction.rollback().unwrap();
    }
}
