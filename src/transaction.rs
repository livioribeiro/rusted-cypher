use std::collections::BTreeMap;
use hyper::Client;
use hyper::header::{Headers, Location};
use serde_json::{self, Value};
use time::{self, Tm};

use super::cypher::{CypherResult, Statement};
use super::error::{GraphError, Neo4jError, TransactionError};

const DATETIME_RFC822: &'static str = "%a, %d %b %Y %T %Z";

#[derive(Debug, Deserialize)]
struct TransactionInfo {
    expires: String,
}

#[derive(Debug, Deserialize)]
struct QueryResult {
    commit: String,
    transaction: TransactionInfo,
    results: Vec<CypherResult>,
    errors: Vec<Neo4jError>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct CommitResult {
    results: Vec<CypherResult>,
    errors: Vec<Neo4jError>,
}

pub struct Transaction<'a> {
    transaction: String,
    commit: String,
    expires: Tm,
    client: Client,
    headers: &'a Headers,
}

fn send_query(client: &Client, endpoint: &str, headers: &Headers, statements: Vec<Statement>)
    -> Result<(QueryResult, String), GraphError> {

    let mut json = BTreeMap::new();
    json.insert("statements", statements);
    let json = try!(serde_json::to_string(&json));

    let req = client.post(endpoint)
        .headers(headers.clone())
        .body(&json);

    let mut res = try!(req.send());

    let result: Value = try!(serde_json::de::from_reader(&mut res));
    let result = try!(serde_json::value::from_value::<QueryResult>(result));

    if result.errors.len() > 0 {
        return Err(GraphError::new_neo4j_error(result.errors))
    }

    let transaction = match res.headers.get::<Location>() {
        Some(location) => location.0.to_owned(),
        None => "".to_owned()
    };

    Ok((result, transaction))
}

impl<'a> Transaction<'a> {
    pub fn begin(endpoint: &str, headers: &'a Headers, statements: Vec<Statement>)
        -> Result<(Self, Vec<CypherResult>), GraphError> {

        let client = Client::new();

        let (result, transaction) = try!(send_query(&client, endpoint, headers, statements));

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

    pub fn is_expired(&self) -> bool {
        self.expires < time::now_utc()
    }

    fn assert_transaction_not_expired(&self) -> Result<(), GraphError> {
        if self.is_expired() {
            let error = TransactionError("Expired Transaction".to_owned());
            return Err(GraphError::new_error(Box::new(error)));
        }
        Ok(())
    }

    pub fn commit(self, statements: Vec<Statement>) -> Result<Vec<CypherResult>, GraphError> {
        try!(self.assert_transaction_not_expired());

        let mut json = BTreeMap::new();
        json.insert("statements", statements);
        let json = try!(serde_json::to_string(&json));

        let req = self.client.post(&self.commit)
            .headers(self.headers.clone())
            .body(&json);

        let mut res = try!(req.send());

        let result: Value = try!(serde_json::de::from_reader(&mut res));
        let result = try!(serde_json::value::from_value::<CommitResult>(result));

        if result.errors.len() > 0 {
            return Err(GraphError::new_neo4j_error(result.errors))
        }

        Ok(result.results)
    }

    pub fn rollback(self) -> Result<(), GraphError> {
        try!(self.assert_transaction_not_expired());

        let req = self.client.delete(&self.transaction).headers(self.headers.clone());
        let mut res = try!(req.send());

        let result: Value = try!(serde_json::de::from_reader(&mut res));
        let result = try!(serde_json::value::from_value::<CommitResult>(result));

        if result.errors.len() > 0 {
            return Err(GraphError::new_neo4j_error(result.errors))
        }

        Ok(())
    }

    pub fn query(&mut self, statements: Vec<Statement>) -> Result<Vec<CypherResult>, GraphError> {
        try!(self.assert_transaction_not_expired());

        let (result, _) = try!(send_query(&self.client, &self.commit, self.headers, statements));

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
    fn create_node_on_begin_transaction_and_commit() {
        let headers = get_headers();
        let params: BTreeMap<String, String> = BTreeMap::new();

        let stmt = Statement::new("create (n:TRANSACTION_1 { name: 'Rust', safe: true })", &params);

        let (transaction, _) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();
        transaction.commit(vec![]).unwrap();

        let stmt = Statement::new("match (n:TRANSACTION_1) return n", &params);
        let (transaction, results) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();

        assert_eq!(results[0].data.len(), 1);
        transaction.commit(vec![]).unwrap();

        let stmt = Statement::new("match (n:TRANSACTION_1) delete n", &params);
        let (transaction, _) = Transaction::begin(URL, &headers, vec![stmt]).unwrap();
        transaction.commit(vec![]).unwrap();
    }
}
