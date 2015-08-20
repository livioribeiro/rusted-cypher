use std::error::Error;
use std::fmt;

#[derive(Debug, Deserialize)]
pub struct Neo4jError {
    pub message: String,
    pub code: String,
}

#[derive(Debug)]
pub struct GraphError {
    pub message: String,
    pub neo4j_errors: Option<Vec<Neo4jError>>,
    pub error: Option<Box<Error>>,
}

impl GraphError {
    pub fn neo4j_error(errors: Vec<Neo4jError>) -> Self {
        GraphError {
            message: "Neo4j Error".to_owned(),
            neo4j_errors: Some(errors),
            error: None,
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
}
