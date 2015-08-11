extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate semver;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use hyper::Url;
use hyper::Client;
use hyper::header::{Authorization, Basic, ContentType, Headers};
use serde_json::Value;
use semver::Version;

#[derive(Debug)]
pub struct Neo4jError {
    message: String,
    code: String,
}

#[derive(Debug)]
pub struct GraphError {
    message: String,
    neo4j_errors: Option<Vec<Neo4jError>>,
    error: Option<Box<Error>>,
}

impl GraphError {
    pub fn neo4j_error(errors: Vec<Neo4jError>) -> Self {
        GraphError {
            message: "".to_owned(),
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

#[derive(Clone)]
pub struct Credentials {
    username: String,
    password: String,
}

#[allow(dead_code)]
pub struct GraphClient {
    endpoint: Url,
    credentials: Option<Credentials>,
    headers: Headers,
    server_params: BTreeMap<String, Value>,
    neo4j_version: Version,
}

impl GraphClient {
    pub fn connect(endpoint: &str) -> Result<Self, Box<Error>> {
        let url = try!(Url::parse(endpoint));
        let mut headers = Headers::new();

        let credentials = url.username()
            .and_then(|username| url.password()
                .and_then(|password| {
                    headers.set(Authorization(
                        Basic {
                            username: username.to_owned(),
                            password: Some(password.to_owned()),
                        }
                    ));
                    Some(Credentials {
                        username: username.to_owned(),
                        password: password.to_owned()
                    })
                })
            );

        headers.set(ContentType::json());

        let client = Client::new();
        let mut res = try!(client.get(url.clone()).headers(headers.clone()).send());

        let result: Value = try!(serde_json::de::from_reader(&mut res));
        result.find("errors").map(|e| {
            let mut errors = Vec::new();
            for error in e.as_array().unwrap() {
                errors.push({
                    Neo4jError {
                        message: error.find("message").unwrap().as_string().unwrap().to_owned(),
                        code: error.find("code").unwrap().as_string().unwrap().to_owned(),
                    }
                });
            }

            return GraphError::neo4j_error(errors)
        });

        let neo4j_version = match result.find("neo4j_version") {
            Some(v) => v.as_string().unwrap(),
            None => return Err(
                Box::new(GraphError {
                    message: "message".to_owned(),
                    neo4j_errors: None,
                    error: None,
                })
            )
        };

        Ok(GraphClient {
            endpoint: url,
            credentials: credentials,
            headers: headers,
            server_params: result.as_object().unwrap().to_owned(),
            neo4j_version: Version::parse(neo4j_version).unwrap(),
        })
    }

    pub fn neo4j_version(&self) -> &Version {
        &self.neo4j_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect() {
        let url = "http://neo4j:blackcat@localhost:7474/db/data";
        let graph = GraphClient::connect(url);
        assert!(graph.is_ok());
        let graph = graph.unwrap();
        assert!(graph.neo4j_version().major >= 2);
    }
}
