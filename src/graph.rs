use std::collections::BTreeMap;
use std::error::Error;
use hyper::{Client, Url};
use hyper::header::{Authorization, Basic, ContentType, Headers};
use serde_json::{self, Value};
use semver::Version;

use cypher::Statements;
use error::{GraphError, Neo4jError};

struct ServiceRoot {
    cypher: Url,
    transaction: Url,
}

#[allow(dead_code)]
pub struct GraphClient {
    client: Client,
    headers: Headers,
    server_params: BTreeMap<String, Value>,
    service_root: ServiceRoot,
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

        let server_params: Value = try!(serde_json::de::from_reader(&mut res));
        server_params.find("errors").map(|e| {
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

        let neo4j_version = match server_params.find("neo4j_version") {
            Some(v) => v.as_string().unwrap(),
            None => return Err(
                Box::new(GraphError {
                    message: "message".to_owned(),
                    neo4j_errors: None,
                    error: None,
                })
            )
        };

        let cypher_endpoint = server_params.find("cypher").unwrap().as_string().unwrap();
        let transaction_endpoint = server_params.find("transaction").unwrap().as_string().unwrap();
        let service_root = ServiceRoot {
            cypher: Url::parse(cypher_endpoint).unwrap(),
            transaction: Url::parse(transaction_endpoint).unwrap(),
        };

        Ok(GraphClient {
            client: Client::new(),
            headers: headers,
            server_params: server_params.as_object().unwrap().to_owned(),
            service_root: service_root,
            neo4j_version: Version::parse(neo4j_version).unwrap(),
        })
    }

    pub fn neo4j_version(&self) -> &Version {
        &self.neo4j_version
    }

    pub fn cypher_query(&self, query: &str) -> Result<BTreeMap<String, Value>, Box<Error>> {
        let mut statements = Statements::new();
        statements.add_stmt(query, BTreeMap::new());
        let json = statements.to_json();
        let json = try!(serde_json::to_string(&json));

        let cypher_commit = format!("{}/{}", self.service_root.transaction, "commit");
        let req = self.client.post(&cypher_commit)
            .headers(self.headers.clone())
            .body(&json);

        let mut res = try!(req.send());

        let result: Value = try!(serde_json::de::from_reader(&mut res));
        Ok(result.as_object().unwrap().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect() {
        let url = "http://neo4j:neo4j@localhost:7474/db/data";
        let graph = GraphClient::connect(url);
        assert!(graph.is_ok());
        let graph = graph.unwrap();
        assert!(graph.neo4j_version().major >= 2);
    }

    #[test]
    fn cypher_query() {
        let url = "http://neo4j:neo4j@localhost:7474/db/data";
        let graph = GraphClient::connect(url).unwrap();

        let result = graph.cypher_query("match n return n");
        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(result.contains_key("results"));
        assert!(result.contains_key("errors"));
    }
}
