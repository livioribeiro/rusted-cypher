use std::collections::BTreeMap;
use std::error::Error;
use rustc_serialize::json::{self, Json};
use hyper::{Client, Url};
use hyper::header::Headers;

struct Statement {
    statement: String,
    parameters: BTreeMap<String, Json>,
}

impl Statement {
    pub fn to_json(self) -> BTreeMap<String, Json> {
        let mut json = BTreeMap::new();
        json.insert("statement".to_owned(), Json::String(self.statement));
        json.insert("parameters".to_owned(), Json::Object(self.parameters));

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

    pub fn add_stmt(&mut self, statement: &str, params: BTreeMap<String, Json>) {
        self.statements.push(Statement {
            statement: statement.to_owned(),
            parameters: params,
        });
    }

    pub fn to_json(self) -> BTreeMap<String, Json> {
        let mut json = BTreeMap::new();
        let mut statements = vec![];
        for s in self.statements {
            statements.push(Json::Object(s.to_json()));
        }

        json.insert("statements".to_owned(), Json::Array(statements));

        json
    }
}

pub struct CypherQuery<'a> {
    statement: String,
    params: BTreeMap<String, Json>,
    cypher: &'a Cypher,
}

impl<'a> CypherQuery<'a> {
    pub fn with_param(&mut self, name: &str, param: Json) -> &mut Self {
        self.params.insert(name.to_owned(), param);
        self
    }

    pub fn with_params(&mut self, params: BTreeMap<String, Json>) {
        self.params = params;
    }

    pub fn send(self, client: &Client, headers: &Headers) -> Result<BTreeMap<String, Json>, Box<Error>> {
        let mut statements = Statements::new();
        statements.add_stmt(&self.statement, self.params);
        let json = statements.to_json();
        let json = try!(json::encode(&json));

        let cypher_commit = format!("{}/{}", self.cypher.endpoint(), "commit");
        let req = client.post(&cypher_commit)
            .headers(headers.clone())
            .body(&json);

        let mut res = try!(req.send());

        let result: Json = try!(Json::from_reader(&mut res));
        Ok(result.as_object().unwrap().to_owned())
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
