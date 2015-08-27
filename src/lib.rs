#![cfg_attr(feature = "serde_macros", feature(custom_derive, plugin))]
#![cfg_attr(feature = "serde_macros", plugin(serde_macros))]

//! Rust crate for accessing the cypher endpoint of a neo4j server
//!
//! This is a prototype for accessing the cypher endpoint of a neo4j server, like a sql
//! driver for a relational database.
//!
//! The main goal of this project is to provide a way to send cypher queries to a neo4j server and
//! retrieve the results.
//! The second goal is to manage transactions through the transaction endpoint.
//!
//! It MAY be extended to support other resources of the neo4j REST api.
//!
//! #Examples
//!
//! ```
//! use std::collections::BTreeMap;
//! use rusted_cypher::GraphClient;
//! use rusted_cypher::cypher::Statement;
//!
//! let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data").unwrap();
//!
//! let mut query = graph.cypher().query();
//! query.add_simple_statement("CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })");
//!
//! let mut params = BTreeMap::new();
//! params.insert("safeness", false);
//! query.add_statement(Statement::new("CREATE (n:LANG { name: 'C++', level: 'low', safe: {safeness} })", &params));
//!
//! query.send().unwrap();
//!
//! graph.cypher().exec("CREATE (n:LANG { name: 'Python', level: 'High', safe: true })").unwrap();
//!
//! let result = graph.cypher().exec("MATCH (n:LANG) RETURN n").unwrap();
//!
//! for row in result.iter() {
//!     println!("{:?}", row);
//! }
//!
//! graph.cypher().exec("MATCH (n:LANG) DELETE n").unwrap();
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
