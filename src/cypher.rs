use std::collections::BTreeMap;
use std::error::Error;
use std::rc::Rc;
use hyper::{Client, Url};
use hyper::header::Headers;
use serde_json::{self, Value};

use super::error::{GraphError, Neo4jError};

#[derive(Serialize)]
pub struct Statement {
    statement: String,
    parameters: BTreeMap<String, Value>,
}

impl Statement {
    pub fn new(statement: &str, parameters: BTreeMap<String, Value>) -> Self {
        Statement {
            statement: statement.to_owned(),
            parameters: parameters,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CypherResult {
    pub columns: Vec<String>,
    pub data: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct QueryResult {
    results: Vec<CypherResult>,
    errors: Vec<Neo4jError>,
}

pub struct CypherQuery<'a> {
    statements: Vec<Statement>,
    cypher: &'a Cypher,
}

impl<'a> CypherQuery<'a> {
    pub fn add_simple_statement(&mut self, statement: &str) {
        self.statements.push(Statement {
            statement: statement.to_owned(),
            parameters: BTreeMap::new(),
        });
    }

    pub fn add_statement(&mut self, statement: Statement) {
        self.statements.push(statement);
    }

    pub fn set_statements(&mut self, statements: Vec<Statement>) {
        self.statements = statements;
    }

    pub fn send(self) -> Result<Vec<CypherResult>, Box<Error>> {
        let client = self.cypher.client.clone();
        let headers = self.cypher.headers.clone();

        let mut json = BTreeMap::new();
        json.insert("statements", self.statements);
        let json = try!(serde_json::to_string(&json));

        let cypher_commit = format!("{}/{}", self.cypher.endpoint(), "commit");
        let req = client.post(&cypher_commit)
            .headers((*headers).to_owned())
            .body(&json);

        let mut res = try!(req.send());

        let result: Value = try!(serde_json::de::from_reader(&mut res));
        match serde_json::value::from_value::<QueryResult>(result) {
            Ok(result) => {
                if result.errors.len() > 0 {
                    return Err(Box::new(GraphError::neo4j_error(result.errors)))
                }

                return Ok(result.results);
            }
            Err(e) => return Err(Box::new(e))
        }
    }
}

pub struct Cypher {
    endpoint: Url,
    client: Rc<Client>,
    headers: Rc<Headers>,
}

impl Cypher {
    pub fn new(endpoint: Url, client: Rc<Client>, headers: Rc<Headers>) -> Self {
        Cypher {
            endpoint: endpoint,
            client: client,
            headers: headers,
        }
    }

    fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    pub fn query(&self) -> CypherQuery {
        CypherQuery {
            statements: Vec::new(),
            cypher: &self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use hyper::{Client, Url};
    use hyper::header::{Authorization, Basic, ContentType, Headers};

    #[test]
    fn query() {
        let cypher_endpoint = Url::parse("http://localhost:7474/db/data/transaction").unwrap();
        let client = Rc::new(Client::new());

        let mut headers = Headers::new();
        headers.set(Authorization(
            Basic {
                username: "neo4j".to_owned(),
                password: Some("neo4j".to_owned()),
            }
        ));
        headers.set(ContentType::json());
        let headers = Rc::new(headers);

        let cypher = Cypher::new(cypher_endpoint, client, headers);
        let mut query = cypher.query();

        query.add_simple_statement("match n return n");

        let result = query.send();
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");
    }
}
