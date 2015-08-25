use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;
use hyper;
use url;
use serde_json;

#[derive(Debug, Deserialize)]
pub struct Neo4jError {
    pub message: String,
    pub code: String,
}

#[derive(Debug)]
pub struct GraphError {
    neo4j_errors: Option<Vec<Neo4jError>>,
    cause: Option<Box<Error>>,
}

impl GraphError {
    pub fn new_neo4j_error(errors: Vec<Neo4jError>) -> Self {
        GraphError {
            neo4j_errors: Some(errors),
            cause: None,
        }
    }

    pub fn new_error(error: Box<Error>) -> Self {
        GraphError {
            neo4j_errors: None,
            cause: Some(error),
        }
    }
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for GraphError {
    fn description(&self) -> &str {
        match self.cause {
            Some(ref cause) => cause.description(),
            None => "Neo4j Error"
        }
    }

    fn cause(&self) -> Option<&Error> {
        match self.cause {
            None => None,
            Some(ref e) => Some(&**e)
        }
    }
}

impl From<FromUtf8Error> for GraphError {
    fn from(error: FromUtf8Error) -> Self {
        GraphError::new_error(Box::new(error))
    }
}

impl From<url::ParseError> for GraphError {
    fn from(error: url::ParseError) -> Self {
        GraphError::new_error(Box::new(error))
    }
}

impl From<hyper::error::Error> for GraphError {
    fn from(error: hyper::error::Error) -> Self {
        GraphError::new_error(Box::new(error))
    }
}

impl From<serde_json::error::Error> for GraphError {
    fn from(error: serde_json::error::Error) -> Self {
        GraphError::new_error(Box::new(error))
    }
}
