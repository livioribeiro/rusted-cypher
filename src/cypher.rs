use std::collections::BTreeMap;
use rustc_serialize::json::Json;

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
