use std::collections::BTreeMap;
use std::error::Error;
use hyper::{Client, Url};
use hyper::header::{Authorization, Basic, ContentType, Headers};
use serde_json::{self, Value};
use semver::Version;

use cypher::Statements;
use error::{GraphError, Neo4jError};

struct ServiceRoot {
    transaction: Url,
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

        let transaction_endpoint = server_params.find("transaction").unwrap().as_string().unwrap();
        let service_root = ServiceRoot {
            transaction: Url::parse(transaction_endpoint).unwrap(),
        };

        Ok(GraphClient {
            client: Client::new(),
            headers: headers,
            service_root: service_root,
            neo4j_version: Version::parse(neo4j_version).unwrap(),
        })
    }

    pub fn neo4j_version(&self) -> &Version {
        &self.neo4j_version
    }

    pub fn cypher_query(&self, statement: &str) -> Result<BTreeMap<String, Value>, Box<Error>> {
        self.cypher_query_params(statement, BTreeMap::new())
    }

    pub fn cypher_query_params(&self, statement: &str, params: BTreeMap<String, Value>) -> Result<BTreeMap<String, Value>, Box<Error>> {
        let mut statements = Statements::new();
        statements.add_stmt(statement, params);
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
    use std::collections::BTreeMap;
    use serde_json::Value;
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
        params.insert("name".to_owned(), Value::String("Neo".to_owned()));

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
