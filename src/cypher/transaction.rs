//! Transaction management through neo4j's transaction endpoint
//!
//! The recommended way to start a transaction is through the `GraphClient`
//!
//! # Examples
//!
//! ## Starting a transaction
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # #[allow(unused_variables)]
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect(URL,None).await?;
//! let mut transaction = graph.transaction();
//! transaction.add_statement("MATCH (n:TRANSACTION) RETURN n");
//!
//! let (transaction, results) = transaction.begin().await?;
//! # transaction.rollback().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Statement is optional when beggining a transaction
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # #[allow(unused_variables)]
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect(URL,None).await?;
//! let (transaction, _) = graph.transaction().begin().await?;
//! # transaction.rollback().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Send queries in a started transaction
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect(URL,None).await?;
//! # let (mut transaction, _) = graph.transaction().begin().await?;
//! // Send a single query
//! let result = transaction.exec("MATCH (n:TRANSACTION) RETURN n").await?;
//!
//! // Send multiple queries
//! let results = transaction
//!     .with_statement("MATCH (n:TRANSACTION) RETURN n")
//!     .with_statement("MATCH (n:OTHER_TRANSACTION) RETURN n")
//!     .send().await?;
//! # assert_eq!(results.len(), 2);
//! # transaction.rollback().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Commit a transaction
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect(URL,None).await?;
//! # let (mut transaction, _) = graph.transaction().begin().await?;
//! transaction.exec("CREATE (n:TRANSACTION)").await?;
//! transaction.commit().await?;
//!
//! // Send more statements when commiting
//! # let (mut transaction, _) = graph.transaction().begin().await?;
//! let results = transaction.with_statement(
//!     "MATCH (n:TRANSACTION) RETURN n")
//!     .send().await?;
//! # assert_eq!(results[0].data.len(), 1);
//! # transaction.rollback().await?;
//! # graph.exec("MATCH (n:TRANSACTION) DELETE n").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Rollback a transaction
//! ```
//! # use rusted_cypher::{GraphClient, GraphError};
//! # const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data";
//! # #[tokio::main]
//! # async fn main() { doctest().await.unwrap(); }
//! # async fn doctest() -> Result<(), GraphError> {
//! # let graph = GraphClient::connect(URL, None).await?;
//! # let (mut transaction, _) = graph.transaction().begin().await?;
//! transaction.exec("CREATE (n:TRANSACTION)").await?;
//! transaction.rollback().await?;
//! # let result = graph.exec("MATCH (n:TRANSACTION) RETURN n").await?;
//! # assert_eq!(result.data.len(), 0);
//! # Ok(())
//! # }
//! ```

use hyper::body::Body;
use hyper::header::{HeaderMap, LOCATION};
use hyper::{client::HttpConnector, Client, Request};
use std::any::Any;
use std::marker::PhantomData;
use std::mem;
use time::{self, Tm};

use super::result::{CypherResult, ResultTrait};
use super::statement::Statement;
use crate::error::{GraphError, Neo4jError};

const DATETIME_RFC822: &'static str = "%a, %d %b %Y %T %Z";

pub struct Created;
pub struct Started;

#[derive(Debug, Deserialize)]
struct TransactionInfo {
  expires: String,
}

#[derive(Debug, Deserialize)]
struct TransactionResult {
  commit: String,
  transaction: TransactionInfo,
  results: Vec<CypherResult>,
  errors: Vec<Neo4jError>,
}

impl ResultTrait for TransactionResult {
  fn results(&self) -> &Vec<CypherResult> {
    &self.results
  }

  fn errors(&self) -> &Vec<Neo4jError> {
    &self.errors
  }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct CommitResult {
  results: Vec<CypherResult>,
  errors: Vec<Neo4jError>,
}

impl ResultTrait for CommitResult {
  fn results(&self) -> &Vec<CypherResult> {
    &self.results
  }

  fn errors(&self) -> &Vec<Neo4jError> {
    &self.errors
  }
}

/// Provides methods to interact with a transaction
///
/// This struct is used to begin a transaction, send queries, commit an rollback a transaction.
/// Some methods are provided depending on the state of the transaction, for example,
/// `Transaction::begin` is provided on a `Created` transaction and `Transaction::commit` is provided
/// on `Started` transaction
pub struct Transaction<State: Any = Created> {
  transaction: String,
  commit: String,
  expires: Tm,
  client: Client<HttpConnector, Body>,
  headers: HeaderMap,
  statements: Vec<Statement>,
  _state: PhantomData<State>,
}

impl<'a, State: Any> Transaction<State> {
  /// Adds a statement to the transaction
  pub fn add_statement<S: Into<Statement>>(&mut self, statement: S) {
    self.statements.push(statement.into());
  }

  /// Gets the expiration time of the transaction
  pub fn get_expires(&self) -> &Tm {
    &self.expires
  }
}

impl Transaction<Created> {
  pub fn new(endpoint: &str, headers: &HeaderMap) -> Transaction<Created> {
    Transaction {
      transaction: endpoint.to_owned(),
      commit: endpoint.to_owned(),
      expires: time::now_utc(),
      client: Client::new(),
      headers: headers.clone(),
      statements: vec![],
      _state: PhantomData,
    }
  }

  /// Adds a statement to the transaction in builder style
  pub fn with_statement<S: Into<Statement>>(mut self, statement: S) -> Self {
    self.add_statement(statement);
    self
  }

  /// Begins the transaction
  ///
  /// Consumes the `Transaction<Created>` and returns the a `Transaction<Started>` alongside with
  /// the results of any `Statement` sent.
  pub async fn begin(self) -> Result<(Transaction<Started>, Vec<CypherResult>), GraphError> {
    debug!("Beginning transaction");

    let res = super::send_query(
      &self.client,
      &self.transaction,
      &self.headers,
      self.statements,
    )
    .await?;
    let headers = res.headers().clone();
    let bytes = hyper::body::to_bytes(res.into_body()).await?;

    let mut result: TransactionResult = super::parse_response(&bytes)?;

    let transaction = headers
      .get(LOCATION)
      .map(|location| location.to_str().unwrap().to_owned())
      .ok_or_else(|| {
        error!("No transaction URI returned from server");
        GraphError::Transaction("No transaction URI returned from server".to_owned())
      })?;

    let expires = time::strptime(&mut result.transaction.expires, DATETIME_RFC822)?;

    debug!(
      "Transaction started at {}, expires in {}",
      transaction,
      expires.rfc822z()
    );

    let transaction = Transaction {
      transaction: transaction,
      commit: result.commit,
      expires: expires,
      client: self.client,
      headers: self.headers,
      statements: Vec::new(),
      _state: PhantomData,
    };

    Ok((transaction, result.results))
  }
}

impl Transaction<Started> {
  /// Adds a statement to the transaction in builder style
  pub fn with_statement<S: Into<Statement>>(&mut self, statement: S) -> &mut Self {
    self.add_statement(statement);
    self
  }

  /// Executes the given statement
  ///
  /// Any statements added via `add_statement` or `with_statement` will be discarded
  pub async fn exec<S: Into<Statement>>(
    &mut self,
    statement: S,
  ) -> Result<CypherResult, GraphError> {
    self.statements.clear();
    self.add_statement(statement);

    let mut results = self.send().await?;
    let result = results.pop().ok_or(GraphError::Statement(
      "Server returned no results".to_owned(),
    ))?;

    Ok(result)
  }

  //   pub async fn exec_single<S: Into<Statement>>(
  //     &mut self,
  //     statement: S,
  //   ) -> Result<CypherResult, GraphError> {
  //     let mut statements = vec![statement];

  //     let res = super::send_query(&self.client, &self.transaction, &self.headers, statements).await?;

  //     let bytes = hyper::body::to_bytes(res.into_body()).await?;

  //     let mut result: TransactionResult = super::parse_response(&bytes)?;
  //     self.expires = time::strptime(&mut result.transaction.expires, DATETIME_RFC822)?;

  //     Ok(result.results)
  //   }

  /// Executes the statements added via `add_statement` or `with_statement`
  pub async fn send(&mut self) -> Result<Vec<CypherResult>, GraphError> {
    let mut statements = vec![];

    mem::swap(&mut statements, &mut self.statements);
    let res = super::send_query(&self.client, &self.transaction, &self.headers, statements).await?;

    let bytes = hyper::body::to_bytes(res.into_body()).await?;

    let mut result: TransactionResult = super::parse_response(&bytes)?;
    self.expires = time::strptime(&mut result.transaction.expires, DATETIME_RFC822)?;

    Ok(result.results)
  }

  /// Commits the transaction, returning the results
  pub async fn commit(self) -> Result<Vec<CypherResult>, GraphError> {
    debug!("Commiting transaction {}", self.transaction);

    let res = super::send_query(&self.client, &self.commit, &self.headers, self.statements).await?;

    let bytes = hyper::body::to_bytes(res.into_body()).await?;

    let result: CommitResult = super::parse_response(&bytes)?;
    debug!("Transaction commited {}", self.transaction);

    Ok(result.results)
  }

  /// Rollback the transaction
  pub async fn rollback(self) -> Result<(), GraphError> {
    debug!("Rolling back transaction {}", self.transaction);

    let mut builder = Request::builder().method("DELETE").uri(&self.transaction);

    for (key, value) in self.headers {
      builder = builder.header(key.unwrap().as_str(), value.to_str().unwrap());
    }

    let req = builder.body(Body::empty()).unwrap();

    let res = self.client.request(req).await?;

    let bytes = hyper::body::to_bytes(res.into_body()).await?;

    super::parse_response::<CommitResult>(&bytes)?;
    debug!("Transaction rolled back {}", self.transaction);

    Ok(())
  }

  /// Sends a query to just reset the transaction timeout
  ///
  /// All transactions have a timeout. Use this method to keep a transaction alive.
  pub async fn reset_timeout(&mut self) -> Result<(), GraphError> {
    super::send_query(&self.client, &self.transaction, &self.headers, vec![])
      .await
      .map(|_| ())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use base64::encode;
  use hyper::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};

  const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data/transaction";

  fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();

    let mut text = String::from("neo4j");
    text.push(':');
    text.push_str("neo4j");
    headers.insert(
      AUTHORIZATION,
      HeaderValue::from_str(&encode(text.as_bytes())).unwrap(),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    headers
  }

  #[tokio::test]
  async fn begin_transaction() {
    let headers = get_headers();
    let transaction = Transaction::new(URL, &headers);
    let result = transaction.begin().await.unwrap();
    assert_eq!(result.1.len(), 0);
  }

  #[tokio::test]
  async fn create_node_and_commit() {
    let headers = get_headers();

    Transaction::new(URL, &headers)
      .with_statement("CREATE (n:TEST_TRANSACTION_CREATE_COMMIT { name: 'Rust', safe: true })")
      .begin()
      .await
      .unwrap()
      .0
      .commit()
      .await
      .unwrap();

    let (transaction, results) = Transaction::new(URL, &headers)
      .with_statement("MATCH (n:TEST_TRANSACTION_CREATE_COMMIT) RETURN n")
      .begin()
      .await
      .unwrap();

    assert_eq!(results[0].data.len(), 1);

    transaction.rollback().await.unwrap();

    Transaction::new(URL, &headers)
      .with_statement("MATCH (n:TEST_TRANSACTION_CREATE_COMMIT) DELETE n")
      .begin()
      .await
      .unwrap()
      .0
      .commit()
      .await
      .unwrap();
  }

  #[tokio::test]
  async fn create_node_and_rollback() {
    let headers = get_headers();

    let (mut transaction, _) = Transaction::new(URL, &headers)
      .with_statement("CREATE (n:TEST_TRANSACTION_CREATE_ROLLBACK { name: 'Rust', safe: true })")
      .begin()
      .await
      .unwrap();

    let result = transaction
      .exec("MATCH (n:TEST_TRANSACTION_CREATE_ROLLBACK) RETURN n")
      .await
      .unwrap();

    assert_eq!(result.data.len(), 1);

    transaction.rollback().await.unwrap();

    let (transaction, results) = Transaction::new(URL, &headers)
      .with_statement("MATCH (n:TEST_TRANSACTION_CREATE_ROLLBACK) RETURN n")
      .begin()
      .await
      .unwrap();

    assert_eq!(results[0].data.len(), 0);

    transaction.rollback().await.unwrap();
  }

  #[tokio::test]
  async fn query_open_transaction() {
    let headers = get_headers();

    let (mut transaction, _) = Transaction::new(URL, &headers).begin().await.unwrap();

    let result = transaction
      .exec("CREATE (n:TEST_TRANSACTION_QUERY_OPEN { name: 'Rust', safe: true }) RETURN n")
      .await
      .unwrap();

    assert_eq!(result.data.len(), 1);

    transaction.rollback().await.unwrap();
  }
}
