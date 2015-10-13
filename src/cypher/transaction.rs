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
//! let stmt = Statement::new("CREATE (n:TRANSACTION)");
//! let (mut transaction, _) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();
//!
//! let stmt = Statement::new("MATCH (n:TRANSACTION) RETURN n");
//! transaction.add_statement(stmt);
//! let results = transaction.exec().unwrap();
//! assert_eq!(results[0].data.len(), 1);
//!
//! transaction.rollback().unwrap();
//! # }
//! ```

use std::convert::Into;
use hyper::header::{Headers, Location};
use hyper::client::Client;
use time::{self, Tm};

use ::error::{GraphError, Neo4jError};
use super::result::{CypherResult, ResultTrait};
use super::statement::Statement;

const DATETIME_RFC822: &'static str = "%a, %d %b %Y %T %Z";

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

pub struct Transaction<'a> {
    transaction: String,
    commit: String,
    expires: Tm,
    client: Client,
    headers: &'a Headers,
    statements: Vec<Statement>,
}

impl<'a> Transaction<'a> {
    pub fn begin(endpoint: &str, headers: &'a Headers, statements: Vec<Statement>)
        -> Result<(Self, Vec<CypherResult>), GraphError> {

        let client = Client::new();

        let mut res = try!(super::send_query(&client, endpoint, headers, statements));
        let result: TransactionResult = try!(super::parse_response(&mut res));

        let transaction = match res.headers.get::<Location>() {
            Some(location) => location.0.to_owned(),
            None => return Err(GraphError::new("No transaction URI returned from server")),
        };

        let mut expires = result.transaction.expires;
        let expires = try!(time::strptime(&mut expires, DATETIME_RFC822));

        let transaction = Transaction {
            transaction: transaction,
            commit: result.commit,
            expires: expires,
            client: Client::new(),
            headers: headers,
            statements: Vec::new(),
        };

        Ok((transaction, result.results))
    }

    pub fn get_expires(&self) -> &Tm {
        &self.expires
    }

    pub fn add_statement<S: Into<Statement>>(&mut self, statement: S) {
        self.statements.push(statement.into());
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
        let mut res = try!(super::send_query(&self.client, &self.commit, self.headers, self.statements));

        let result: CommitResult = try!(super::parse_response(&mut res));

        Ok(result.results)
    }

    pub fn rollback(self) -> Result<(), GraphError> {
        let req = self.client.delete(&self.transaction).headers(self.headers.clone());
        let mut res = try!(req.send());

        try!(super::parse_response::<CommitResult>(&mut res));

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
    use ::Statement;
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
        let result = Transaction::begin(URL, &headers, vec![]).unwrap();
        assert_eq!(result.1.len(), 0);
    }

    #[test]
    fn create_node_and_commit() {
        let headers = get_headers();

        let stmt = Statement::new("create (n:CREATE_COMMIT { name: 'Rust', safe: true })");
        let (transaction, _) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();
        transaction.commit().unwrap();

        let stmt = Statement::new("match (n:CREATE_COMMIT) return n");
        let (transaction, results) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();

        assert_eq!(results[0].data.len(), 1);
        transaction.commit().unwrap();

        let stmt = Statement::new("match (n:CREATE_COMMIT) delete n");
        let (transaction, _) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();
        transaction.commit().unwrap();
    }

    #[test]
    fn create_node_and_rollback() {
        let headers = get_headers();

        let stmt = Statement::new("create (n:CREATE_ROLLBACK { name: 'Rust', safe: true })");
        let (transaction, _) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();
        transaction.rollback().unwrap();

        let stmt = Statement::new("match (n:CREATE_ROLLBACK) return n");
        let (transaction, results) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();

        assert_eq!(results[0].data.len(), 0);
        transaction.commit().unwrap();
    }

    #[test]
    fn query_open_transaction() {
        let headers = get_headers();

        let (mut transaction, _) = Transaction::begin(URL, &headers, vec![]).unwrap();

        let stmt = Statement::new("create (n:QUERY_OPEN { name: 'Rust', safe: true }) return n");
        transaction.add_statement(stmt);
        let results = transaction.exec().unwrap();

        assert_eq!(results[0].data.len(), 1);

        transaction.rollback().unwrap();
    }
}
