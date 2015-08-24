use std::error::Error;
use std::fmt;

#[derive(Debug, Deserialize)]
pub struct Neo4jError {
    pub message: String,
    pub code: String,
}

#[derive(Debug)]
pub struct GraphError {
    message: String,
    neo4j_errors: Option<Vec<Neo4jError>>,
    cause: Option<Box<Error>>,
}

impl GraphError {
    pub fn new(message: &str, neo4j_errors: Option<Vec<Neo4jError>>, cause: Option<Box<Error>>) -> Self {
        GraphError {
            message: message.to_owned(),
            neo4j_errors: neo4j_errors,
            cause: cause,
        }
    }
    pub fn new_neo4j_error(errors: Vec<Neo4jError>) -> Self {
        GraphError {
            message: "Neo4j Error".to_owned(),
            neo4j_errors: Some(errors),
            cause: None,
        }
    }

    pub fn new_error(error: Box<Error>) -> Self {
        GraphError {
            message: error.description().to_owned(),
            neo4j_errors: None,
            cause: Some(error),
        }
    }
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for GraphError {
    fn description(&self) -> &str {
        &self.message
    }

    fn cause<'a>(&'a self) -> Option<&'a Error> {
        match self.cause {
            None => None,
            Some(ref e) => Some(&**e)
        }
    }
}
