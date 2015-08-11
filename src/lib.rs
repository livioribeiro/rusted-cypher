extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate semver;

use std::collections::BTreeMap;
use std::error::Error;
use hyper::{Client, Url};
use hyper::header::{Authorization, Basic, ContentType, Headers};
use serde_json::Value;
use semver::Version;

mod error;

use error::{GraphError, Neo4jError};

#[derive(Clone)]
pub struct Credentials {
    username: String,
    password: String,
}

#[allow(dead_code)]
pub struct GraphClient {
    endpoint: Url,
    headers: Headers,
    server_params: BTreeMap<String, Value>,
    neo4j_version: Version,
}

impl GraphClient {
    pub fn connect(endpoint: &str) -> Result<Self, Box<Error>> {
        let url = try!(Url::parse(endpoint));
        let mut headers = Headers::new();

        url.username().map(|username| url.password().map(|password| {
            headers.set(Authorization(
                Basic {
                    username: username.to_owned(),
                    password: Some(password.to_owned()),
                }
            ));
        }));

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
