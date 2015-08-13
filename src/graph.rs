use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, Read};
use hyper::{Client, Url};
use hyper::header::{Authorization, Basic, ContentType, Headers};
use rustc_serialize::json::{self, Json};
use semver::Version;

use cypher::Statements;
use error::{GraphError, Neo4jError};

#[derive(RustcDecodable)]
#[allow(dead_code)]
struct ServiceRoot {
    node: String,
    node_index: String,
    relationship_index: String,
    extensions_info: String,
    relationship_types: String,
    batch: String,
    cypher: String,
    indexes: String,
    constraints: String,
    transaction: String,
    node_labels: String,
    neo4j_version: String,
}

pub struct GraphClient {
    client: Client,
    headers: Headers,
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
                    None => return Err(Box::new(io::Error::new(io::ErrorKind::Other, "Somethind wrong happened")))
                }
            }
        };

        let neo4j_version = Version::parse(&service_root.neo4j_version).unwrap();

        Ok(GraphClient {
            client: Client::new(),
            headers: headers,
            service_root: service_root,
            neo4j_version: neo4j_version,
        })
    }

    pub fn neo4j_version(&self) -> &Version {
        &self.neo4j_version
    }

    pub fn cypher_query(&self, statement: &str) -> Result<BTreeMap<String, Json>, Box<Error>> {
        self.cypher_query_params(statement, BTreeMap::new())
    }

    pub fn cypher_query_params(&self, statement: &str, params: BTreeMap<String, Json>) -> Result<BTreeMap<String, Json>, Box<Error>> {
        let mut statements = Statements::new();
        statements.add_stmt(statement, params);
        let json = statements.to_json();
        let json = try!(json::encode(&json));

        let cypher_commit = format!("{}/{}", self.service_root.transaction, "commit");
        let req = self.client.post(&cypher_commit)
            .headers(self.headers.clone())
            .body(&json);

        let mut res = try!(req.send());

        let result: Json = try!(Json::from_reader(&mut res));
        Ok(result.as_object().unwrap().to_owned())
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

        assert!(result.contains_key("results"));
        assert!(result.contains_key("errors"));

        let errors = result["errors"].as_array().unwrap();
        assert!(errors.len() == 0);
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

        assert!(result.contains_key("results"));
        assert!(result.contains_key("errors"));

        let errors = result["errors"].as_array().unwrap();
        assert!(errors.len() == 0, format!("errors: {:?}", errors));
    }
}
