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

impl CypherResult {
    pub fn iter(&self) -> Iter {
        Iter::new(&self.data)
    }
}

pub struct IntoIter(Vec<Value>);

impl Iterator for IntoIter {
    type Item = Value;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop().map(
            |item| item.find("row").expect("Wrong response: Missing 'row' property").to_owned()
        )
    }
}

pub struct Iter<'a> {
    current_index: usize,
    data: &'a Vec<Value>,
}

impl<'a> Iter<'a> {
    pub fn new(data: &'a Vec<Value>) -> Self {
        Iter {
            current_index: 0_usize,
            data: data,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Vec<Value>;
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.data.get(self.current_index);
        item.map(|i| {
            self.current_index += 1;
            i.find("row").unwrap().as_array().unwrap()
        })
    }
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

    const URL: &'static str = "http://localhost:7474/db/data/transaction";

    fn get_cypher() -> Cypher {
        let cypher_endpoint = Url::parse(URL).unwrap();
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

        Cypher::new(cypher_endpoint, client, headers)
    }

    #[test]
    fn query() {
        let cypher = get_cypher();
        let mut query = cypher.query();

        query.add_simple_statement("match n return n");

        let result = query.send().unwrap();

        assert_eq!(result[0].columns.len(), 1);
        assert_eq!(result[0].columns[0], "n");
    }

    #[test]
    fn into_iter() {
        let cypher = get_cypher();
        let mut query = cypher.query();

        query.add_simple_statement(
            "create (n {name: 'Name', lastname: 'LastName'}), (m {name: 'Name', lastname: 'LastName'})");

        query.send().unwrap();

        let mut query = cypher.query();
        query.add_simple_statement("match n return n");

        let result = query.send().unwrap();

        assert_eq!(result[0].data.len(), 2);

        let result = result.get(0).unwrap().to_owned();
        for row in result.iter() {
            assert!(row[0].find("name").is_some());
            assert!(row[0].find("lastname").is_some());
            assert_eq!(row[0].find("name").unwrap().as_string().unwrap(), "Name");
            assert_eq!(row[0].find("lastname").unwrap().as_string().unwrap(), "LastName");
        }

        let mut query = cypher.query();
        query.add_simple_statement("match n delete n");
        query.send().unwrap();
    }
}
