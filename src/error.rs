//! Error types returned by the `GraphClient`

use hyper::{self, http::uri::InvalidUri};
use semver::SemVerError;
use serde_json;
use std::error::Error;
use std::fmt;
use std::io;
use std::string::FromUtf8Error;
use thiserror::Error;
use time;

#[derive(Clone, Debug, Deserialize)]
pub struct Neo4jError {
  pub message: String,
  pub code: String,
}

#[derive(Debug)]
pub struct TimeParseError(time::ParseError, String);

impl fmt::Display for TimeParseError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl Error for TimeParseError {
  fn description(&self) -> &str {
    &self.1
  }
}

#[derive(Debug, Error)]
pub enum GraphError {
  #[error("Errors Neo4j {0:?}")]
  Neo4j(Vec<Neo4jError>),
  #[error("Statement {0}")]
  Statement(String),
  #[error("Transaction {0}")]
  Transaction(String),
  #[error("IO/Error {0}")]
  Io(#[from] io::Error),
  #[error("FromUtf8")]
  FromUtf8(#[from] FromUtf8Error),
  #[error("InvalidUri")]
  InvalidUri(#[from] InvalidUri),
  //UrlParse(#[from] hyper::error::Error),
  #[error("Hyper {0}")]
  Hyper(#[from] hyper::Error),
  #[error("Serde {0}")]
  Serde(#[from] serde_json::Error),
  #[error("TimeParse {0}")]
  TimeParse(#[from] time::ParseError),
  #[error("SemVerError {0}")]
  SemVer(#[from] SemVerError),
  #[error("OtherError {0}")]
  Other(String),
}

// quick_error! {
//     #[derive(Debug)]
//     pub enum GraphError {
//         Neo4j(err: Vec<Neo4jError>) {
//             from()
//         }
//         Statement(err: String)
//         Transaction(err: String)
//         Io(err: io::Error) {
//             from()
//         }
//         FromUtf8(err: FromUtf8Error) {
//             from()
//         }
//         UrlParse(err: hyper::error::Error) {
//             from()
//         }
//         Hyper(err: hyper::Error) {
//             from()
//         }
//         Serde(err: serde_json::Error) {
//             from()
//         }
//         TimeParse(err: time::ParseError) {
//             from()
//         }
//         SemVer(err: SemVerError) {
//             from()
//         }
//         Other(err: String) {
//             from()
//         }
//     }
// }
