//! Main interface for interacting with a neo4j server

use std::collections::BTreeMap;
use std::io::Read;
use hyper::{Client, Url};
use hyper::header::{Authorization, Basic, ContentType, Headers};
use serde_json::{self, Value};
use semver::Version;

use cypher::Cypher;
use error::GraphError;
use cypher::result::QueryResult;

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

fn decode_service_root<R: Read>(reader: &mut R) -> Result<ServiceRoot, GraphError> {
    let mut bytes: Vec<u8> = vec![];
    reader.read_to_end(&mut bytes)?;

    let result = serde_json::de::from_slice::<ServiceRoot>(&bytes);

    result.map_err(|_| {
        match serde_json::de::from_slice::<QueryResult>(&bytes) {
            Ok(result) => GraphError::Neo4j(result.errors),
            Err(e) => From::from(e),
        }
    })
}

#[allow(dead_code)]
pub struct GraphClient {
    headers: Headers,
    service_root: ServiceRoot,
    neo4j_version: Version,
    cypher: Cypher,
}

impl GraphClient {
    pub fn connect<T: AsRef<str>>(endpoint: T) -> Result<Self, GraphError> {
        let endpoint = endpoint.as_ref();
        let url = Url::parse(endpoint)
            .map_err(|e| {
                error!("Unable to parse URL");
                e
            })?;

        let mut headers = Headers::new();

        url.password().map(|password| {
            headers.set(Authorization(
                Basic {
                    username: url.username().to_owned(),
                    password: Some(password.to_owned()),
                }
            ));
        });

        headers.set(ContentType::json());

        let client = Client::new();
        let mut res = client.get(endpoint)
            .headers(headers.clone())
            .send()
            .map_err(|e| {
                error!("Unable to connect to server: {}", &e);
                e
            })?;

        let service_root = decode_service_root(&mut res)?;

        let neo4j_version = Version::parse(&service_root.neo4j_version)?;
        let cypher_endpoint = Url::parse(&service_root.transaction)?;

        let cypher = Cypher::new(cypher_endpoint, client, headers.clone());

        Ok(GraphClient {
            headers: headers,
            service_root: service_root,
            neo4j_version: neo4j_version,
            cypher: cypher,
        })
    }

    pub fn neo4j_version(&self) -> &Version {
        &self.neo4j_version
    }

    /// Returns a reference to the `Cypher` instance of the `GraphClient`
    pub fn cypher(&self) -> &Cypher {
        &self.cypher
    }
}

#[cfg(test)]
mod tests {
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
    fn query() {
        let graph = GraphClient::connect(URL).unwrap();

        let mut query = graph.cypher().query();
        query.add_statement("MATCH (n) RETURN n");

        let result = query.send().unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");
    }

    #[test]
    fn transaction() {
        let graph = GraphClient::connect(URL).unwrap();

        let (transaction, result) = graph.cypher().transaction()
            .with_statement("MATCH (n) RETURN n")
            .begin()
            .unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");

        transaction.rollback().unwrap();
    }
}
