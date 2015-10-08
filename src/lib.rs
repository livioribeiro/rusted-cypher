#![cfg_attr(feature = "serde_macros", feature(custom_derive, plugin))]
#![cfg_attr(feature = "serde_macros", plugin(serde_macros))]

//! Rust crate for accessing the cypher endpoint of a neo4j server
//!
//! This is a prototype for accessing the cypher endpoint of a neo4j server, like a sql
//! driver for a relational database.
//!
//! You can execute queries inside a transaction or simply execute queries that commit immediately.
//!
//! It MAY be extended to support other resources of the neo4j REST api.
//!
//! # Examples
//!
//! Code in examples are assumed to be wrapped in:
//!
//! ```
//! extern crate rusted_cypher;
//!
//! use std::collections::BTreeMap;
//! use rusted_cypher::GraphClient;
//! use rusted_cypher::cypher::Statement;
//!
//! fn main() {
//!   // Connect to the database
//!   let graph = GraphClient::connect(
//!       "http://neo4j:neo4j@localhost:7474/db/data").unwrap();
//!
//!   // Example code here!
//! }
//! ```
//!
//! ## Performing Queries
//!
//! ```
//! # use std::collections::BTreeMap;
//! # use rusted_cypher::GraphClient;
//! # use rusted_cypher::cypher::Statement;
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data").unwrap();
//! let mut query = graph.cypher().query();
//!
//! // Statement implements From<&str>
//! query.add_statement(
//!     "CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })");
//!
//! let mut statement = Statement::new(
//!     "CREATE (n:LANG { name: 'C++', level: 'low', safe: {safeness} })");
//! statement.add_param("safeness", false);
//!
//! query.add_statement(statement);
//!
//! query.send().unwrap();
//!
//! graph.cypher().exec(
//!     "CREATE (n:LANG { name: 'Python', level: 'high', safe: true })"
//! ).unwrap();
//!
//! let result = graph.cypher().exec("MATCH (n:LANG) RETURN n").unwrap();
//!
//! for row in result.iter() {
//!     println!("{:?}", row);
//! }
//!
//! graph.cypher().exec("MATCH (n:LANG) DELETE n").unwrap();
//! ```
//!
//! ## With Transactions
//!
//! ```
//! # use std::collections::BTreeMap;
//! # use rusted_cypher::GraphClient;
//! # use rusted_cypher::cypher::Statement;
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data").unwrap();
//! let stmt = Statement::new(
//!     "CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })");
//!
//! let (mut transaction, results)
//!     = graph.cypher().begin_transaction(vec![stmt]).unwrap();
//!
//! let stmt = Statement::new(
//!     "CREATE (n:LANG { name: 'Python', level: 'high', safe: true })");
//!
//! transaction.add_statement(stmt);
//! transaction.exec().unwrap();
//!
//! let mut params = BTreeMap::new();
//! params.insert("safeness", true);
//!
//! let mut stmt = Statement::new("MATCH (n:LANG) WHERE (n.safe = {safeness}) RETURN n");
//! stmt.add_param("safeness", true);
//!
//! transaction.add_statement(stmt);
//! let results = transaction.exec().unwrap();
//!
//! assert_eq!(results[0].data.len(), 2);
//!
//! transaction.rollback();
//! ```

extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate semver;
extern crate time;
extern crate url;

#[cfg(feature = "serde_macros")]
include!("lib.rs.in");

#[cfg(not(feature = "serde_macros"))]
include!(concat!(env!("OUT_DIR"), "/lib.rs"));
