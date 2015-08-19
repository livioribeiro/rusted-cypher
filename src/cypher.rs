use std::collections::BTreeMap;
use std::error::Error;
use hyper::{Client, Url};
use hyper::header::Headers;
use serde_json::{self, Value};

use super::error::{GraphError, Neo4jError};

struct Statement {
    statement: String,
    parameters: BTreeMap<String, Value>,
}

impl Statement {
    pub fn to_json(self) -> BTreeMap<String, Value> {
        let mut json = BTreeMap::new();
        json.insert("statement".to_owned(), Value::String(self.statement));
        json.insert("parameters".to_owned(), Value::Object(self.parameters));

        json
    }
}

pub struct Statements {
    statements: Vec<Statement>,
}

impl Statements {
    pub fn new() -> Self {
        Statements {
            statements: Vec::new(),
        }
    }

    pub fn add_stmt(&mut self, statement: &str, params: BTreeMap<String, Value>) {
        self.statements.push(Statement {
            statement: statement.to_owned(),
            parameters: params,
        });
    }

    pub fn to_json(self) -> BTreeMap<String, Value> {
        let mut json = BTreeMap::new();
        let mut statements = vec![];
        for s in self.statements {
            statements.push(Value::Object(s.to_json()));
        }

        json.insert("statements".to_owned(), Value::Array(statements));

        json
    }
}

#[derive(Debug)]
pub struct CypherResult {
    pub columns: Vec<String>,
    pub data: Vec<Value>,
}

pub struct CypherQuery<'a> {
    statement: String,
    params: BTreeMap<String, Value>,
    cypher: &'a Cypher,
}

impl<'a> CypherQuery<'a> {
    pub fn with_param(&mut self, name: &str, param: Value) -> &mut Self {
        self.params.insert(name.to_owned(), param);
        self
    }

    pub fn with_params(&mut self, params: BTreeMap<String, Value>) {
        self.params = params;
    }

    pub fn send(self, client: &Client, headers: &Headers) -> Result<Vec<CypherResult>, Box<Error>> {
        let mut statements = Statements::new();
        statements.add_stmt(&self.statement, self.params);
        let json = statements.to_json();
        let json = try!(serde_json::to_string(&json));

        let cypher_commit = format!("{}/{}", self.cypher.endpoint(), "commit");
        let req = client.post(&cypher_commit)
            .headers(headers.clone())
            .body(&json);

        let mut res = try!(req.send());

        let result: Value = try!(serde_json::de::from_reader(&mut res));
        let errors = result.find("errors").unwrap().as_array().unwrap();

        if errors.len() > 0 {
            let mut error_list = Vec::new();
            for error in errors {
                let message = error.find("message").unwrap().as_string().unwrap();
                let code = error.find("code").unwrap().as_string().unwrap();

                error_list.push(Neo4jError { message: message.to_string(), code: code.to_string() });
            }

            return Err(Box::new(GraphError::neo4j_error(error_list)));
        }

        let mut cypher_result = Vec::new();
        for result in result.find("results").unwrap().as_array().unwrap() {
            let mut columns = Vec::new();
            for column in result.find("columns").unwrap().as_array().unwrap() {
                columns.push(column.as_string().unwrap().to_owned());
            }

            let data = result.find("data").unwrap().as_array().unwrap();

            cypher_result.push(CypherResult { columns: columns, data: data.to_owned() });
        }

        Ok(cypher_result)
    }
}

pub struct Cypher {
    endpoint: Url,
}

impl Cypher {
    pub fn new(endpoint: Url) -> Self {
        Cypher {
            endpoint: endpoint,
        }
    }

    fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    pub fn query(&self, statement: &str) -> CypherQuery {
        CypherQuery {
            statement: statement.to_owned(),
            params: BTreeMap::new(),
            cypher: &self,
        }
    }
}
