#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

//! Rust crate for accessing the cypher endpoint of a neo4j server
//!
//! This crate is a prototype for a client for the cypher endpoint of a neo4j server, like a sql
//! driver for a relational database.
//! The goal of this project is to provide a way to send cypher queries to a neo4j server and
//! iterate over the results. It MAY be extended to support other resources of the neo4j REST api.

extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate semver;

pub mod cypher;
pub mod graph;
pub mod error;

pub use graph::GraphClient;
