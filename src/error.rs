//! Error types returned by the `GraphClient`

use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;
use std::io;
use hyper;
use serde_json;
use time;
use semver::SemVerError;

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

quick_error! {
    #[derive(Debug)]
    pub enum GraphError {
        Neo4j(err: Vec<Neo4jError>) {
            from()
        }
        Statement(err: String)
        Transaction(err: String)
        Io(err: io::Error) {
            from()
        }
        FromUtf8(err: FromUtf8Error) {
            from()
        }
        UrlParse(err: hyper::error::ParseError) {
            from()
        }
        Hyper(err: hyper::Error) {
            from()
        }
        Serde(err: serde_json::Error) {
            from()
        }
        TimeParse(err: time::ParseError) {
            from()
        }
        SemVer(err: SemVerError) {
            from()
        }
        Other(err: String) {
            from()
        }
    }
}
