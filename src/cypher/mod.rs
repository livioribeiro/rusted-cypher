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
//! query.add_statement("match n return n");
//!
//! let result = query.send().unwrap();
//!
//! for row in result.iter() {
//!     println!("{:?}", row);
//! }
//! # }
//! ```


pub mod cypher;
pub mod transaction;
pub mod statement;

pub use self::cypher::Cypher;
pub use self::cypher::CypherQuery;
pub use self::cypher::CypherResult;
pub use self::statement::Statement;
pub use self::transaction::Transaction;
