//! Transaction management through neo4j's transaction endpoint
//!
//! # Examples
//!
//! ```
//! # extern crate hyper;
//! # extern crate rusted_cypher;
//! # use hyper::header::{Authorization, Basic, ContentType, Headers};
//! # use rusted_cypher::Statement;
//! # use rusted_cypher::cypher::Transaction;
//! # fn main() {
//! # const URL: &'static str = "http://neo4j:neo4j@localhost:7474/db/data/transaction";
//! # let mut headers = Headers::new();
//! # headers.set(Authorization(
//! #     Basic {
//! #         username: "neo4j".to_owned(),
//! #         password: Some("neo4j".to_owned()),
//! #     }
//! # ));
//! # headers.set(ContentType::json());
//! let transaction = Transaction::new(URL, &headers)
//!     .with_statement("CREATE (n:TRANSACTION)");
//!
//! let (mut transaction, _) = transaction.begin().unwrap();
//!
//! transaction.add_statement("MATCH (n:TRANSACTION) RETURN n");
//! let results = transaction.exec().unwrap();
//! assert_eq!(results[0].data.len(), 1);
//!
//! transaction.rollback().unwrap();
//! # }
//! ```

use std::any::Any;
use std::convert::Into;
use std::marker::PhantomData;
use hyper::Client;
use hyper::header::{Headers, Location};
use time::{self, Tm};

use ::error::{GraphError, Neo4jError};
use super::result::{CypherResult, ResultTrait};
use super::statement::Statement;

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

pub struct Transaction<'a, State: Any = Created> {
    transaction: String,
    commit: String,
    expires: Tm,
    client: Client,
    headers: &'a Headers,
    statements: Vec<Statement>,
    _state: PhantomData<State>,
}

impl<'a, State: Any> Transaction<'a, State> {
    pub fn add_statement<S: Into<Statement>>(&mut self, statement: S) {
        self.statements.push(statement.into());
    }

    pub fn get_expires(&self) -> &Tm {
        &self.expires
    }
}

impl<'a> Transaction<'a, Created> {
    pub fn new(endpoint: &str, headers: &'a Headers) -> Transaction<'a, Created> {
        Transaction {
            transaction: endpoint.to_owned(),
            commit: endpoint.to_owned(),
            expires: time::now_utc(),
            client: Client::new(),
            headers: headers,
            statements: vec![],
            _state: PhantomData,
        }
    }

    pub fn with_statement<S: Into<Statement>>(mut self, statement: S) -> Self {
        self.add_statement(statement);
        self
    }

    pub fn begin(self) -> Result<(Transaction<'a, Started>, Vec<CypherResult>), GraphError> {
        debug!("Beginning transaction");

        let mut res = try!(super::send_query(&self.client,
                                             &self.transaction,
                                             self.headers,
                                             self.statements));

        let result: TransactionResult = try!(super::parse_response(&mut res));

        let transaction = match res.headers.get::<Location>() {
            Some(location) => location.0.to_owned(),
            None => {
                error!("No transaction URI returned from server");
                return Err(GraphError::new("No transaction URI returned from server"));
            },
        };

        let mut expires = result.transaction.expires;
        let expires = try!(time::strptime(&mut expires, DATETIME_RFC822));

        debug!("Transaction started at {}, expires in {}", transaction, expires.rfc822z());

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

impl<'a> Transaction<'a, Started> {
    pub fn with_statement<S: Into<Statement>>(&mut self, statement: S) -> &mut Self {
        self.add_statement(statement);
        self
    }

    pub fn exec(&mut self) -> Result<Vec<CypherResult>, GraphError> {
        let mut res = try!(super::send_query(&self.client, &self.transaction, self.headers, self.statements.clone()));
        self.statements.clear();

        let result: TransactionResult = try!(super::parse_response(&mut res));

        let mut expires = result.transaction.expires;
        let expires = try!(time::strptime(&mut expires, DATETIME_RFC822));

        self.expires = expires;

        Ok(result.results)
    }

    pub fn commit(self) -> Result<Vec<CypherResult>, GraphError> {
        debug!("Commiting transaction {}", self.transaction);
        let mut res = try!(super::send_query(&self.client, &self.commit, self.headers, self.statements));

        let result: CommitResult = try!(super::parse_response(&mut res));
        debug!("Transaction commited {}", self.transaction);

        Ok(result.results)
    }

    pub fn rollback(self) -> Result<(), GraphError> {
        debug!("Rolling back transaction {}", self.transaction);
        let req = self.client.delete(&self.transaction).headers(self.headers.clone());
        let mut res = try!(req.send());

        try!(super::parse_response::<CommitResult>(&mut res));
        debug!("Transaction rolled back {}", self.transaction);

        Ok(())
    }

    pub fn reset_timeout(&mut self) -> Result<(), GraphError> {
        try!(super::send_query(&self.client, &self.transaction, self.headers, vec![]));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::{Authorization, Basic, ContentType, Headers};

    const URL: &'static str = "http:neo4j:neo4j@localhost:7474/db/data/transaction";

    fn get_headers() -> Headers {
        let mut headers = Headers::new();

        headers.set(Authorization(
            Basic {
                username: "neo4j".to_owned(),
                password: Some("neo4j".to_owned()),
            }
        ));

        headers.set(ContentType::json());

        headers
    }

    #[test]
    fn begin_transaction() {
        let headers = get_headers();
        let transaction = Transaction::new(URL, &headers);
        let result = transaction.begin().unwrap();
        assert_eq!(result.1.len(), 0);
    }

    #[test]
    fn create_node_and_commit() {
        let headers = get_headers();

        let transaction = Transaction::new(URL, &headers)
            .with_statement("CREATE (n:TEST_TRANSACTION_CREATE_COMMIT { name: 'Rust', safe: true })");
        let (transaction, _) = transaction.begin().unwrap();
        transaction.commit().unwrap();

        let transaction = Transaction::new(URL, &headers)
            .with_statement("MATCH (n:TEST_TRANSACTION_CREATE_COMMIT) RETURN n");
        let (transaction, results) = transaction.begin().unwrap();

        assert_eq!(results[0].data.len(), 1);
        transaction.commit().unwrap();

        let transaction = Transaction::new(URL, &headers)
            .with_statement("MATCH (n:TEST_TRANSACTION_CREATE_COMMIT) DELETE n");
        let (transaction, _) = transaction.begin().unwrap();
        transaction.commit().unwrap();
    }

    #[test]
    fn create_node_and_rollback() {
        let headers = get_headers();

        let transaction = Transaction::new(URL, &headers)
            .with_statement("CREATE (n:TEST_TRANSACTION_CREATE_ROLLBACK { name: 'Rust', safe: true })");
        let (transaction, _) = transaction.begin().unwrap();
        transaction.rollback().unwrap();

        let transaction = Transaction::new(URL, &headers)
            .with_statement("MATCH (n:TEST_TRANSACTION_CREATE_ROLLBACK) RETURN n");
        let (transaction, results) = transaction.begin().unwrap();

        assert_eq!(results[0].data.len(), 0);
        transaction.commit().unwrap();
    }

    #[test]
    fn query_open_transaction() {
        let headers = get_headers();

        let (mut transaction, _) = Transaction::new(URL, &headers).begin().unwrap();

        let results = transaction
            .with_statement("CREATE (n:TEST_TRANSACTION_QUERY_OPEN { name: 'Rust', safe: true }) RETURN n")
            .exec()
            .unwrap();

        assert_eq!(results[0].data.len(), 1);

        transaction.rollback().unwrap();
    }
}
