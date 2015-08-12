use std::collections::BTreeMap;
use serde_json::Value;

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
