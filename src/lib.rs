//! Rust crate for accessing the cypher endpoint of a neo4j server
//!
//! This crate allows you to send cypher queries to the REST endpoint of a neo4j database. You can
//! execute queries inside a transaction or simply send queries that commit immediately.
//!
//! # Examples
//!
//! ## Connecting to a Neo4j database
//!
//! ```
//! use rusted_cypher::GraphClient;
//! let graph = GraphClient::connect(
//!     "http://neo4j:neo4j@localhost:7474/db/data");
//! # graph.unwrap();
//! ```
//!
//! ## Performing Queries
//!
//! ```
//! # use rusted_cypher::{GraphClient, Statement, GraphError};
//! # fn main() { doctest().unwrap(); }
//! # fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data")?;
//! let mut query = graph.query();
//!
//! // Statement implements From<&str>
//! query.add_statement(
//!     "CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })");
//!
//! let statement = Statement::new(
//!     "CREATE (n:LANG { name: 'C++', level: 'low', safe: {safeness} })")
//!     .with_param("safeness", false)?;
//!
//! query.add_statement(statement);
//!
//! query.send()?;
//!
//! graph.exec(
//!     "CREATE (n:LANG { name: 'Python', level: 'high', safe: true })")?;
//!
//! let result = graph.exec(
//!     "MATCH (n:LANG) RETURN n.name, n.level, n.safe")?;
//!
//! assert_eq!(result.data.len(), 3);
//!
//! for row in result.rows() {
//!     let name: String = row.get("n.name")?;
//!     let level: String = row.get("n.level")?;
//!     let safeness: bool = row.get("n.safe")?;
//!     println!("name: {}, level: {}, safe: {}", name, level, safeness);
//! }
//!
//! graph.exec("MATCH (n:LANG) DELETE n")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Transactions
//!
//! ```
//! # use std::collections::BTreeMap;
//! # use rusted_cypher::{GraphClient, Statement, GraphError};
//! # fn main() { doctest().unwrap(); }
//! # fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data")?;
//! let transaction = graph
//!     .transaction()
//!     .with_statement(
//!         "CREATE (n:IN_TRANSACTION { name: 'Rust', level: 'low', safe: true })");
//!
//! let (mut transaction, results) = transaction.begin().unwrap();
//!
//! // Use `exec` to execute a single statement
//! transaction.exec("CREATE (n:IN_TRANSACTION { name: 'Python', level: 'high', safe: true })")?;
//!
//! // use `add_statement` (or `with_statement`) and `send` to executes multiple statements
//! let stmt = Statement::new(
//!     "MATCH (n:IN_TRANSACTION) WHERE (n.safe = {safeness}) RETURN n")
//!     .with_param("safeness", true)?;
//!
//! transaction.add_statement(stmt);
//! let results = transaction.send()?;
//!
//! assert_eq!(results[0].data.len(), 2);
//!
//! transaction.rollback()?;
//! # Ok(())
//! }
//! ```
//!
//! ## Statements with Macro
//!
//! There is a macro to help building statements
//!
//! ```
//! # #[macro_use] extern crate rusted_cypher;
//! # use rusted_cypher::{GraphClient, Statement, GraphError};
//! # fn main() { doctest().unwrap(); }
//! # fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data")?;
//! let statement = cypher_stmt!(
//!     "CREATE (n:WITH_MACRO { name: {name}, level: {level}, safe: {safe} })", {
//!         "name" => "Rust",
//!         "level" => "low",
//!         "safe" => true
//!     }
//! )?;
//! graph.exec(statement)?;
//!
//! let statement = cypher_stmt!(
//!     "MATCH (n:WITH_MACRO) WHERE n.name = {name} RETURN n", {
//!         "name" => "Rust"
//!     }
//! )?;
//!
//! let results = graph.exec(statement)?;
//! assert_eq!(results.data.len(), 1);
//!
//! let statement = cypher_stmt!("MATCH (n:WITH_MACRO) DELETE n")?;
//! graph.exec(statement)?;
//! # Ok(())
//! # }
//! ```

extern crate hyper;
pub extern crate serde;
pub extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate semver;
extern crate time;

#[macro_use]
extern crate quick_error;

#[macro_use]
extern crate log;

pub mod cypher;
pub mod graph;
pub mod error;

pub use graph::GraphClient;
pub use cypher::Statement;
pub use error::GraphError;
