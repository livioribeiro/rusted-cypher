use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use hyper::{Client, Url};
use hyper::header::{Authorization, Basic, ContentType, Headers};
use serde_json::{self, Value};
use semver::Version;

use cypher::Cypher;
use error::{GraphError, Neo4jError};

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ServiceRoot {
    pub extensions: BTreeMap<String, Value>,
    pub node: String,
    pub node_index: String,
    pub relationship_index: String,
    pub extensions_info: String,
    pub relationship_types: String,
    pub batch: String,
    pub cypher: String,
    pub indexes: String,
    pub constraints: String,
    pub transaction: String,
    pub node_labels: String,
    pub neo4j_version: String,
}

fn decode_service_root(json_string: &str) -> Result<ServiceRoot, GraphError> {
    let service_root: ServiceRoot = match serde_json::de::from_str(json_string) {
        Ok(value) => value,
        Err(e) => {
            let error_response: Value = try!(serde_json::de::from_str(json_string));
            match error_response.find("errors") {
                Some(e) => {
                    let mut errors = Vec::new();
                    for error in e.as_array().unwrap() {
                        errors.push({
                            Neo4jError {
                                message: error.find("message").unwrap().as_string().unwrap().to_owned(),
                                code: error.find("code").unwrap().as_string().unwrap().to_owned(),
                            }
                        });
                    }
                    return Err(GraphError::new_neo4j_error(errors))
                },
                None => return Err(GraphError::new_error(Box::new(e)))
            }
        }
    };
    Ok(service_root)
}

#[allow(dead_code)]
pub struct GraphClient {
    client: Client,
    headers: Headers,
    service_root: ServiceRoot,
    neo4j_version: Version,
    cypher: Cypher,
}

impl GraphClient {
    /// Connects to the neo4j server
    pub fn connect(endpoint: &str) -> Result<Self, GraphError> {
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
        let mut buf = String::new();
        match res.read_to_string(&mut buf) {
            Err(e) => return Err(GraphError::new_error(Box::new(e))),
            _ => {}
        }

        let service_root = try!(decode_service_root(&buf));

        let neo4j_version = match Version::parse(&service_root.neo4j_version) {
            Ok(value) => value,
            Err(e) => return Err(GraphError::new_error(Box::new(e))),
        };
        let cypher_endpoint = try!(Url::parse(&service_root.transaction));

        let cypher = Cypher::new(cypher_endpoint, headers.clone());

        Ok(GraphClient {
            client: Client::new(),
            headers: headers,
            service_root: service_root,
            neo4j_version: neo4j_version,
            cypher: cypher,
        })
    }

    pub fn neo4j_version(&self) -> &Version {
        &self.neo4j_version
    }

    /// Gets a reference to the `Cypher` instance of the `GraphClient`
    pub fn cypher(&self) -> &Cypher {
        &self.cypher
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::Statement;

    const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";

    #[test]
    fn connect() {
        let graph = GraphClient::connect(URL);
        assert!(graph.is_ok());
        let graph = graph.unwrap();
        assert!(graph.neo4j_version().major >= 2);
    }

    #[test]
    fn query() {
        let graph = GraphClient::connect(URL).unwrap();

        let mut query = graph.cypher().query();
        query.add_statement("match n return n");

        let result = query.send().unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");
    }

    #[test]
    fn transaction() {
        let graph = GraphClient::connect(URL).unwrap();

        let statement = Statement::new("match n return n");
        let (transaction, result) = graph.cypher().begin_transaction(vec![statement]).unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");

        transaction.rollback().unwrap();
    }
}
