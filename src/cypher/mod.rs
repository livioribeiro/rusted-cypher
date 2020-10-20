//! Provides structs used to interact with the cypher transaction endpoint

pub mod transaction;
pub mod statement;
pub mod result;

use crate::GraphError;
use serde_json::json;

pub use self::statement::Statement;
pub use self::transaction::Transaction;
pub use self::result::CypherResult;

use hyper::{Request, Response, Uri, body::Bytes, client::HttpConnector};
use hyper::body::Body;
use hyper::client::{Client};
use hyper::header::HeaderMap;
use serde::de::DeserializeOwned;
use serde_json::{self, Value};
use serde_json::de as json_de;
use serde_json::ser as json_ser;
use serde_json::value as json_value;


use self::result::{QueryResult, ResultTrait};


async fn send_query(client: &Client<HttpConnector, Body>, endpoint: &str, headers: &HeaderMap, statements: Vec<Statement>)
    -> Result<Response<Body>, GraphError> {

    
    let data = json!({"statements": statements});
    let json = serde_json::to_string(&data)?;


    let mut builder = Request::builder()
         .method("POST")
         .uri(endpoint);

    for (key, value) in headers {
        builder = builder.header(key.as_str(), value.to_str().unwrap());
    }
         
    let req  = builder.body( Body::from(json)).unwrap();


    debug!("Sending query:\n{}", json_ser::to_string_pretty(&data).unwrap_or(String::new()));
    let req = client.request(req);
    let result = req.await?;
    Ok(result)
}

fn parse_response<T: DeserializeOwned + ResultTrait>(bytes: &Bytes) -> Result<T, GraphError> {
    let result: Value = json_de::from_slice(bytes)?;

    if let Some(errors) = result.get("errors") {
        if errors.as_array().map(|a| a.len()).unwrap_or(0) > 0 {
            return Err(GraphError::Neo4j(json_value::from_value(errors.clone())?))
        }
    }

    json_value::from_value::<T>(result).map_err(|e| {
        error!("Unable to parse response: {}", &e);
        From::from(e)
    })
}

/// Represents the cypher endpoint of a neo4j server
///
/// The `Cypher` struct holds information about the cypher enpoint. It is used to create the queries
/// that are sent to the server.
pub struct Cypher {
    endpoint: Uri,
    client: Client<HttpConnector, Body>,
    headers: HeaderMap,
}

impl Cypher {
    /// Creates a new Cypher
    ///
    /// Its arguments are the cypher transaction endpoint and the HTTP headers containing HTTP
    /// Basic Authentication, if needed.
    pub fn new(endpoint: Uri, client: Client<HttpConnector, Body>, headers: HeaderMap) -> Self {
        Cypher {
            endpoint: endpoint,
            client: client,
            headers: headers,
        }
    }

    fn endpoint_commit(&self) -> String {
        format!("{}/{}", &self.endpoint, "commit")
    }

    fn client(&self) -> &Client<HttpConnector, Body> {
        &self.client
    }

    fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Creates a new `CypherQuery`
    pub fn query(&self) -> CypherQuery {
        CypherQuery {
            statements: Vec::new(),
            cypher: &self,
        }
    }

    /// Executes the given `Statement`
    ///
    /// Parameter can be anything that implements `Into<Statement>`, `Into<String>` or `Statement`
    /// itself
    pub async fn exec<S: Into<Statement>>(&self, statement: S) -> Result<CypherResult, GraphError> {
        self.query()
            .with_statement(statement)
            .send().await?
            .pop()
            .ok_or(GraphError::Other("No results returned from server".to_owned()))
    }

    /// Creates a new `Transaction`
    pub fn transaction(&self) -> Transaction<self::transaction::Created> {
        Transaction::new(&self.endpoint.to_string(), &self.headers)
    }
}

/// Represents a cypher query
///
/// A cypher query is composed by statements, each one containing the query itself and its
/// parameters.
///
/// The query parameters must implement `Serialize` so they can be serialized into JSON in order to
/// be sent to the server
pub struct CypherQuery<'a> {
    statements: Vec<Statement>,
    cypher: &'a Cypher,
}

impl<'a> CypherQuery<'a> {
    /// Adds statements in builder style
    pub fn with_statement<T: Into<Statement>>(mut self, statement: T) -> Self {
        self.add_statement(statement);
        self
    }

    pub fn add_statement<T: Into<Statement>>(&mut self, statement: T) {
        self.statements.push(statement.into());
    }

    pub fn statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn set_statements(&mut self, statements: Vec<Statement>) {
        self.statements = statements;
    }

    /// Sends the query to the server
    ///
    /// The statements contained in the query are sent to the server and the results are parsed
    /// into a `Vec<CypherResult>` in order to match the response of the neo4j api.
    pub async fn send(self) -> Result<Vec<CypherResult>, GraphError> {


        let  res = send_query(self.cypher.client(),
                   &self.cypher.endpoint_commit(),
                   self.cypher.headers(),
                   self.statements).await?;

        let bytes = hyper::body::to_bytes(res.into_body()).await?;

        let result: QueryResult = parse_response(&bytes)?;
        if result.errors().len() > 0 {
            return Err(GraphError::Neo4j(result.errors().clone()))
        }

        Ok(result.results)
    }
}

#[cfg(test)]
mod tests {
        use hyper::header::HeaderValue;
        use base64::encode;
        use hyper::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};

use super::*;
    use crate::cypher::result::Row;

    fn get_cypher() -> Cypher  {
        
        let cypher_endpoint = "http://localhost:7474/db/data/transaction".parse::<Uri>().unwrap();

        let mut headers = HeaderMap::new();

        let mut text = String::from("neo4j");
        text.push(':');
        text.push_str("neo4j");
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&encode(text.as_bytes())).unwrap());
        headers.insert( CONTENT_TYPE, HeaderValue::from_static("application/json"));

        Cypher::new(cypher_endpoint, Client::new(), headers)
    }

    #[tokio::test]
    async fn query_without_params() {
        let result = get_cypher().exec("MATCH (n:TEST_CYPHER) RETURN n").await.unwrap();

        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.columns[0], "n");
    }

    #[tokio::test]
    async fn query_with_string_param() {
        let statement = Statement::new("MATCH (n:TEST_CYPHER {name: $name}) RETURN n")
            .with_param("name", "Neo").unwrap();

        let result = get_cypher().exec(statement).await.unwrap();

        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.columns[0], "n");
    }

    #[tokio::test]
    async fn query_with_int_param() {
        let statement = Statement::new("MATCH (n:TEST_CYPHER {value: $value}) RETURN n")
            .with_param("value", 42).unwrap();

        let result = get_cypher().exec(statement).await.unwrap();

        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.columns[0], "n");
    }

    #[tokio::test]
    async fn query_with_complex_param() {
        #[derive(Serialize, Deserialize)]
        pub struct ComplexType {
            pub name: String,
            pub value: i32,
        }

        let cypher = get_cypher();

        let complex_param = ComplexType {
            name: "Complex".to_owned(),
            value: 42,
        };

        let statement = Statement::new("CREATE (n:TEST_CYPHER_COMPLEX_PARAM $p)")
            .with_param("p", &complex_param).unwrap();

        let result = cypher.exec(statement).await;
        assert!(result.is_ok());

        let results = cypher.exec("MATCH (n:TEST_CYPHER_COMPLEX_PARAM) RETURN n").await.unwrap();
        let rows: Vec<Row> = results.rows().take(1).collect();
        let row = rows.first().unwrap();

        let complex_result: ComplexType = row.get("n").unwrap();
        assert_eq!(complex_result.name, "Complex");
        assert_eq!(complex_result.value, 42);

        cypher.exec("MATCH (n:TEST_CYPHER_COMPLEX_PARAM) DELETE n").await.unwrap();
    }

    #[tokio::test]
    async fn query_with_multiple_params() {
        let statement = Statement::new(
            "MATCH (n:TEST_CYPHER {name: $name}) WHERE n.value = $value RETURN n")
            .with_param("name", "Neo").unwrap()
            .with_param("value", 42).unwrap();

        let result = get_cypher().exec(statement).await.unwrap();
        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.columns[0], "n");
    }

    #[tokio::test]
    async fn multiple_queries() {
        let cypher = get_cypher();
        let statement1 = Statement::new("MATCH (n:TEST_CYPHER) RETURN n");
        let statement2 = Statement::new("MATCH (n:TEST_CYPHER) RETURN n");

        let query = cypher.query()
            .with_statement(statement1)
            .with_statement(statement2);

        let results = query.send().await.unwrap();
        assert_eq!(results.len(), 2);
    }
}
