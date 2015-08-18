use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use hyper::{Client, Url};
use hyper::header::{Authorization, Basic, ContentType, Headers};
use rustc_serialize::json::{self, Json};
use semver::Version;

use cypher::{Cypher, CypherResult};
use error::{GraphError, Neo4jError};

#[derive(RustcDecodable)]
#[allow(dead_code)]
pub struct ServiceRoot {
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

pub struct GraphClient {
    client: Client,
    headers: Headers,
    service_root: ServiceRoot,
    neo4j_version: Version,
    cypher: Cypher,
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
        let mut buf = String::new();
        try!(res.read_to_string(&mut buf));

        let service_root: ServiceRoot = match json::decode(&buf) {
            Ok(value) => value,
            Err(_) => {
                let error_response = try!(Json::from_str(&buf));
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
                        return Err(Box::new(GraphError::neo4j_error(errors)))
                    },
                    None => return Err(Box::new(
                        GraphError { message: "Something wrong happened".to_owned(), neo4j_errors: None, error: None}
                    ))
                }
            }
        };

        let neo4j_version = try!(Version::parse(&service_root.neo4j_version));
        let cypher_endpoint = try!(Url::parse(&service_root.transaction));

        Ok(GraphClient {
            client: Client::new(),
            headers: headers,
            service_root: service_root,
            neo4j_version: neo4j_version,
            cypher: Cypher::new(cypher_endpoint)
        })
    }

    pub fn neo4j_version(&self) -> &Version {
        &self.neo4j_version
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_headers(&self) -> Headers {
        self.headers.clone()
    }

    pub fn get_service_root(&self) -> &ServiceRoot {
        &self.service_root
    }

    pub fn cypher_query(&self, statement: &str) -> Result<Vec<CypherResult>, Box<Error>> {
        self.cypher_query_params(statement, BTreeMap::new())
    }

    pub fn cypher_query_params(&self, statement: &str, params: BTreeMap<String, Json>) -> Result<Vec<CypherResult>, Box<Error>> {
        let mut query = self.cypher.query(statement);
        query.with_params(params);

        query.send(&self.client, &self.headers)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use rustc_serialize::json::Json;
    use super::*;

    const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";

    #[test]
    fn connect() {
        let graph = GraphClient::connect(URL);
        assert!(graph.is_ok());
        let graph = graph.unwrap();
        assert!(graph.neo4j_version().major >= 2);
    }

    #[test]
    fn cypher_query() {
        let graph = GraphClient::connect(URL).unwrap();

        let result = graph.cypher_query("match n return n");
        assert!(result.is_ok());
        let result = result.unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");
    }

    #[test]
    fn cypher_query_params() {
        let graph = GraphClient::connect(URL).unwrap();

        let mut params = BTreeMap::new();
        params.insert("name".to_owned(), Json::String("Neo".to_owned()));

        let result = graph.cypher_query_params(
            "match (n {name: {name}}) return n", params);

        assert!(result.is_ok());
        let result = result.unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");
    }
}
