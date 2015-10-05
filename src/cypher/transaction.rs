//! Transaction management through neo4j's transaction endpoint
//!
//! # Examples
//!
//! ```
//! # extern crate hyper;
//! # extern crate rusted_cypher;
//! # use std::collections::BTreeMap;
//! # use hyper::Url;
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
//! let params: BTreeMap<String, String> = BTreeMap::new();
//! let stmt = Statement::new("CREATE (n:TRANSACTION)", &params);
//!
//! let (mut transaction, _) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();
//!
//! let stmt = Statement::new("MATCH (n:TRANSACTION) RETURN n", &params);
//! let results = transaction.exec(vec![stmt]).unwrap();
//! assert_eq!(results[0].data.len(), 1);
//!
//! transaction.rollback().unwrap();
//! # }
//! ```

use std::collections::BTreeMap;
use hyper::Client;
use hyper::client::response::Response;
use hyper::header::{Headers, Location};
use serde::Deserialize;
use serde_json::{self, Value};
use time::{self, Tm};

use ::error::{GraphError, Neo4jError};
use super::cypher::CypherResult;
use super::statement::Statement;

const DATETIME_RFC822: &'static str = "%a, %d %b %Y %T %Z";

#[derive(Debug, Deserialize)]
struct TransactionInfo {
    expires: String,
}

trait ResultTrait {
    fn results(&self) -> &Vec<CypherResult>;
    fn errors(&self) -> &Vec<Neo4jError>;
}

#[derive(Debug, Deserialize)]
struct QueryResult {
    commit: String,
    transaction: TransactionInfo,
    results: Vec<CypherResult>,
    errors: Vec<Neo4jError>,
}

impl ResultTrait for QueryResult {
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
}

fn send_query(client: &Client, endpoint: &str, headers: &Headers, statements: Vec<Statement>)
    -> Result<Response, GraphError> {

    let mut json = BTreeMap::new();
    json.insert("statements", statements);
    let json = try!(serde_json::to_string(&json));

    let req = client.post(endpoint)
        .headers(headers.clone())
        .body(&json);

    let res = try!(req.send());
    Ok(res)
}

fn parse_response<T: Deserialize + ResultTrait>(res: &mut Response) -> Result<T, GraphError> {
    let mut res = res;
    let value: Value = try!(serde_json::de::from_reader(&mut res));
    let result = try!(serde_json::value::from_value::<T>(value.clone()));

    if result.errors().len() > 0 {
        return Err(GraphError::new_neo4j_error(result.errors().clone()));
    }

    Ok(result)
}

impl<'a> Transaction<'a> {
    pub fn begin(endpoint: &str, headers: &'a Headers, statements: Vec<Statement>)
        -> Result<(Self, Vec<CypherResult>), GraphError> {

        let client = Client::new();

        let mut res = try!(send_query(&client, endpoint, headers, statements));
        let result: QueryResult = try!(parse_response(&mut res));

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
        };

        Ok((transaction, result.results))
    }

    pub fn get_expires(&self) -> &Tm {
        &self.expires
    }

    pub fn commit(self, statements: Vec<Statement>) -> Result<Vec<CypherResult>, GraphError> {
        let mut res = try!(send_query(&self.client, &self.commit, self.headers, statements));

        let result: CommitResult = try!(parse_response(&mut res));

        Ok(result.results)
    }

    pub fn rollback(self) -> Result<(), GraphError> {
        let req = self.client.delete(&self.transaction).headers(self.headers.clone());
        let mut res = try!(req.send());

        try!(parse_response::<CommitResult>(&mut res));

        Ok(())
    }

    pub fn exec(&mut self, statements: Vec<Statement>) -> Result<Vec<CypherResult>, GraphError> {
        let mut res = try!(send_query(&self.client, &self.transaction, self.headers, statements));
        let result: QueryResult = try!(parse_response(&mut res));

        let mut expires = result.transaction.expires;
        let expires = try!(time::strptime(&mut expires, DATETIME_RFC822));

        self.expires = expires;

        Ok(result.results)
    }

    pub fn reset_timeout(&mut self) -> Result<(), GraphError> {
        try!(send_query(&self.client, &self.transaction, self.headers, vec![]));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::Statement;
    use std::collections::BTreeMap;
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
        Transaction::begin(URL, &headers, vec![]).unwrap();
    }

    #[test]
    fn create_node_and_commit() {
        let headers = get_headers();
        let params: BTreeMap<String, String> = BTreeMap::new();

        let stmt = Statement::new("create (n:CREATE_COMMIT { name: 'Rust', safe: true })", &params);

        let (transaction, _) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();
        transaction.commit(vec![]).unwrap();

        let stmt = Statement::new("match (n:CREATE_COMMIT) return n", &params);
        let (transaction, results) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();

        assert_eq!(results[0].data.len(), 1);
        transaction.commit(vec![]).unwrap();

        let stmt = Statement::new("match (n:CREATE_COMMIT) delete n", &params);
        let (transaction, _) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();
        transaction.commit(vec![]).unwrap();
    }

    #[test]
    fn create_node_and_rollback() {
        let headers = get_headers();
        let params: BTreeMap<String, String> = BTreeMap::new();

        let stmt = Statement::new("create (n:CREATE_ROLLBACK { name: 'Rust', safe: true })", &params);

        let (transaction, _) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();
        transaction.rollback().unwrap();

        let stmt = Statement::new("match (n:CREATE_ROLLBACK) return n", &params);
        let (transaction, results) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();

        assert_eq!(results[0].data.len(), 0);
        transaction.commit(vec![]).unwrap();
    }

    #[test]
    fn query_open_transaction() {
        let headers = get_headers();
        let params: BTreeMap<String, String> = BTreeMap::new();

        let (mut transaction, _) = Transaction::begin(URL, &headers, vec![]).unwrap();

        let stmt = Statement::new("create (n:QUERY_OPEN { name: 'Rust', safe: true }) return n", &params);
        let results = transaction.exec(vec![stmt]).unwrap();

        assert_eq!(results[0].data.len(), 1);

        transaction.rollback().unwrap();
    }
}
