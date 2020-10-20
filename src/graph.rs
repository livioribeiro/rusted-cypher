//! Main interface for executing queries against a neo4j database
//!
//! # Examples
//!
//! ## Execute a single query
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data", None).await?;
//! graph.exec("CREATE (n:CYPHER_QUERY {value: 1})").await?;
//!
//! let result = graph.exec("MATCH (n:CYPHER_QUERY) RETURN n.value AS value").await?;
//! # assert_eq!(result.data.len(), 1);
//!
//! // Iterate over the results
//! for row in result.rows() {
//!     let value = row.get::<i32>("value")?; // or: let value: i32 = row.get("value")?;
//!     assert_eq!(value, 1);
//! }
//! # graph.exec("MATCH (n:CYPHER_QUERY) delete n").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Execute multiple queries
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data", None).await?;
//! let mut query = graph.query()
//!     .with_statement("MATCH (n:SOME_CYPHER_QUERY) RETURN n.value as value")
//!     .with_statement("MATCH (n:OTHER_CYPHER_QUERY) RETURN n");
//!
//! let results = query.send().await?;
//!
//! for row in results[0].rows() {
//!     let value: i32 = row.get("value")?;
//!     assert_eq!(value, 1);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Start a transaction
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data", None).await?;
//! let (transaction, results) = graph.transaction()
//!     .with_statement("MATCH (n:TRANSACTION_CYPHER_QUERY) RETURN n")
//!     .begin().await?;
//! # assert_eq!(results.len(), 1);
//! # Ok(())
//! }
//! ```

use std::collections::BTreeMap;
use base64::encode;
use hyper::{Body, Client, Request, StatusCode, Uri, body::Bytes};
use hyper::header::HeaderMap;
use serde_json::{self, Value};
use serde_json::value as json_value;
use semver::Version;


use crate::cypher::{Cypher, CypherQuery, CypherResult};
use crate::cypher::transaction::{Transaction, Created as TransactionCreated};
use crate::cypher::statement::Statement;
use crate::error::GraphError;
use regex::Regex;


#[derive(Deserialize)]
pub struct ServiceRoot {
    pub extensions: Option<BTreeMap<String, Value>>,
    pub node: Option<String>,
    pub node_index: Option<String>,
    pub relationship_index: Option<String>,
    pub extensions_info: Option<String>,
    pub relationship_types: Option<String>,
    pub batch: Option<String>,
    pub cypher: Option<String>,
    pub indexes: Option<String>,
    pub constraints: Option<String>,
    pub transaction: String,
    pub node_labels: Option<String>,
    pub neo4j_version: String,
    pub neo4j_edition: Option<String>,
    pub bolt_direct: Option<String>,
}

fn decode_service_root(bytes: &Bytes) -> Result<ServiceRoot, GraphError> {
    //let result: Value = serde_json::de::from_reader(reader)?;
    let result: Value = serde_json::de::from_slice(bytes)?;

    if let Some(errors) = result.get("errors") {
        if errors.as_array().map(|a| a.len()).unwrap_or(0) > 0 {
            return Err(GraphError::Neo4j(json_value::from_value(errors.clone())?))
        }
    }

    json_value::from_value(result)
        .map_err(From::from)
}

#[allow(dead_code)]
pub struct GraphClient {
    headers: HeaderMap,
    service_root: ServiceRoot,
    neo4j_version: Version,
    cypher: Cypher,
}

impl GraphClient {
    pub async fn connect<T: AsRef<str>>(endpoint: T, database: Option<String>) -> Result<Self, GraphError> {
        let endpoint = endpoint.as_ref();
        let uri =  endpoint.parse::<Uri>()
            .map_err(|e| {
                error!("Unable to parse URL");
                e
            })?;

        let mut builder = Request::builder()
            .method("GET")
            .uri(uri.clone());
        
        if let Some(authority) = uri.authority() { 
            let mut parts = authority.as_str().splitn(2,"@");
            let text = parts.next().unwrap();
            if  let Some(_) = parts.next() { 
                builder = builder.header ("authorization", &encode(text.as_bytes()));
            }
        }    

        
        // match uri.authority() {
        //     Some(authority) => {
        //         let mut parts = authority.as_str().splitn(2,"@");
        //         let text = parts.next().unwrap();
        //         match parts.next() { 
        //             Some(_) => {
        //                 builder = builder.header ("authorization", &encode(text.as_bytes()));
        //             },
        //             None => ()
        //         }
        //     },

        //     None => {}
        // }

        // match uri.authority().map( |authority| {
        //     let mut parts = authority.as_str().splitn(2,"@");
        //     let text = parts.next().unwrap();
        //     parts.next().map(|s| {
        //         builder.header (AUTHORIZATION.as_str(), &encode(text.as_bytes()))
        //         //headers.insert(AUTHORIZATION, HeaderValue::from_str(&encode(text.as_bytes())).unwrap());
        //     })
        // })



        builder = builder.header ( "content-type", "application/json");

        let client = Client::new();
         
        let req  = builder.body( Body::empty()).unwrap();
       
        let headers = req.headers().clone();
        let result = client.request(req).await;
        let res = result?;

        let should_redirect = match res.status()  {
            StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND | StatusCode::SEE_OTHER => {
                true
            },
            StatusCode::TEMPORARY_REDIRECT|StatusCode::PERMANENT_REDIRECT => {
                true
            },
            _ => false,
        };
        
        let bytes = if should_redirect && res.headers().contains_key("location")  {
            let location = res.headers().get("location").unwrap().to_str().unwrap();
            let mut new_uri = location.parse::<Uri>().unwrap();
            if new_uri.query() == uri.query() { 
                let parsed_str = format!("{}://{}:{}", new_uri.scheme().unwrap(), new_uri.host().unwrap(), new_uri.port().unwrap() );
                new_uri = parsed_str.parse::<Uri>().unwrap();
            }

            builder = Request::builder()
                .method("GET")
                .uri(new_uri);
            for (k,v) in headers.clone() {
                builder = builder.header ( k.unwrap().as_str(), v.to_str().unwrap());
            }

            let req  = builder.body( Body::empty()).unwrap();
    
            let result = client.request(req).await;
            let res = result?;
            hyper::body::to_bytes(res.into_body()).await?
        } else {
            hyper::body::to_bytes(res.into_body()).await?
        };        

        let service_root_result = decode_service_root(&bytes);
        let service_root = service_root_result?;

        let neo4j_version = Version::parse(&service_root.neo4j_version)?;

        let re = Regex::new(r"(\{databaseName\})").unwrap();
        let database_name = database.unwrap_or_else(||String::from("neo4j"));
        let transaction_endpoint = re.replace_all(&service_root.transaction[..], &database_name[..]);


        let cypher_endpoint = transaction_endpoint.parse::<Uri>()?;

        let cypher = Cypher::new(cypher_endpoint, client, headers.clone());

        Ok(GraphClient {
            headers: headers,
            service_root: service_root,
            neo4j_version: neo4j_version,
            cypher: cypher,
        })
    }

    /// Creates a new `CypherQuery`
    pub fn query(&self) -> CypherQuery {
        self.cypher.query()
    }

    /// Executes the given `Statement`
    ///
    /// Parameter can be anything that implements `Into<Statement>`, `Into<String>` or `Statement`
    /// itself
    pub async fn exec<S: Into<Statement>>(&self, statement: S) -> Result<CypherResult, GraphError> {
        self.cypher.exec(statement).await
    }

    /// Creates a new `Transaction`
    pub fn transaction(&self) -> Transaction<TransactionCreated> {
        self.cypher.transaction()
    }

    pub fn neo4j_version(&self) -> &Version {
        &self.neo4j_version
    }

    pub fn neo4j_edition(&self) -> Option<String> {
        self.service_root.neo4j_edition.clone()
    }

    /// Returns a reference to the `Cypher` instance of the `GraphClient`
    #[deprecated(since = "1.0.0", note = "Use methods on `GraphClient` instead")]
    pub fn cypher(&self) -> &Cypher {
        &self.cypher
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";

    #[tokio::test]
    async fn connect() {
        let graph = GraphClient::connect(URL, None).await;
        assert!(graph.is_ok());
        let graph = graph.unwrap();
        assert!(graph.neo4j_version().major >= 2);
    }

    #[tokio::test]
    async fn query() {
        let graph = GraphClient::connect(URL, None).await.unwrap();

        let mut query = graph.query();
        query.add_statement("MATCH (n) RETURN n");

        let result = query.send().await.unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");
    }

    #[tokio::test]
    async fn transaction() {
        let graph = GraphClient::connect(URL, None).await.unwrap();

        let (transaction, result) = graph.transaction()
            .with_statement("MATCH (n) RETURN n")
            .begin()
            .await
            .unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");

        transaction.rollback().await.unwrap();
    }
}
