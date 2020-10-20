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
//!     "http://neo4j:neo4j@localhost:7474/db/data", None);
//! ```
//!
//! ## Performing Queries
//!
//! ```
//! # use rusted_cypher::{GraphClient, Statement, GraphError};
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data", None).await?;
//! let mut query = graph.query();
//!
//! // Statement implements From<&str>
//! query.add_statement(
//!     "CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })");
//!
//! let statement = Statement::new(
//!     "CREATE (n:LANG { name: 'C++', level: 'low', safe: $safeness })")
//!     .with_param("safeness", false)?;
//!
//! query.add_statement(statement);
//!
//! query.send().await?;
//!
//! graph.exec(
//!     "CREATE (n:LANG { name: 'Python', level: 'high', safe: true })").await?;
//!
//! let result = graph.exec(
//!     "MATCH (n:LANG) RETURN n.name, n.level, n.safe").await?;
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
//! graph.exec("MATCH (n:LANG) DELETE n").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Transactions
//!
//! ```
//! # use std::collections::BTreeMap;
//! # use rusted_cypher::{GraphClient, Statement, GraphError};
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data", None).await?;
//! let transaction = graph
//!     .transaction()
//!     .with_statement(
//!         "CREATE (n:IN_TRANSACTION { name: 'Rust', level: 'low', safe: true })");
//!
//! let (mut transaction, results) = transaction.begin().await.unwrap();
//!
//! // Use `exec` to execute a single statement
//! transaction.exec("CREATE (n:IN_TRANSACTION { name: 'Python', level: 'high', safe: true })").await?;
//!
//! // use `add_statement` (or `with_statement`) and `send` to executes multiple statements
//! let stmt = Statement::new(
//!     "MATCH (n:IN_TRANSACTION) WHERE (n.safe = $safeness) RETURN n")
//!     .with_param("safeness", true)?;
//!
//! transaction.add_statement(stmt);
//! let results = transaction.send().await?;
//!
//! assert_eq!(results[0].data.len(), 2);
//!
//! transaction.rollback().await?;
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
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data", None).await?;
//! let statement = cypher_stmt!(
//!     "CREATE (n:WITH_MACRO { name: $name, level: $level, safe: $safe })", {
//!         "name" => "Rust",
//!         "level" => "low",
//!         "safe" => true
//!     }
//! )?;
//! graph.exec(statement).await?;
//!
//! let statement = cypher_stmt!(
//!     "MATCH (n:WITH_MACRO) WHERE n.name = $name RETURN n", {
//!         "name" => "Rust"
//!     }
//! )?;
//!
//! let results = graph.exec(statement).await?;
//! assert_eq!(results.data.len(), 1);
//!
//! let statement = cypher_stmt!("MATCH (n:WITH_MACRO) DELETE n")?;
//! graph.exec(statement).await?;
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
extern crate base64;

extern crate thiserror;

extern crate tokio;

#[macro_use]
extern crate log;

pub mod cypher;
pub mod graph;
pub mod error;

pub use graph::GraphClient;
pub use cypher::Statement;
pub use error::GraphError;
