extern crate serde;

#[macro_use]
extern crate serde_derive;

extern crate rusted_cypher;

use rusted_cypher::{GraphClient, Statement};
use rusted_cypher::cypher::result::Row;

const URI: &'static str = "http://neo4j:neo4j@127.0.0.1:7474/db/data";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Language {
    name: String,
    level: String,
    safe: bool,
}

#[test]
fn save_retrieve_struct() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI).unwrap();

    let statement = Statement::new("CREATE (n:NTLY_INTG_TEST_1 {lang}) RETURN n")
        .with_param("lang", &rust);

    let results = graph.cypher().exec(statement).unwrap();
    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    graph.cypher().exec("MATCH (n:NTLY_INTG_TEST_1) DELETE n").unwrap();
}

#[test]
fn transaction_create_on_begin_commit() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:NTLY_INTG_TEST_2 {lang})")
        .with_param("lang", &rust);

    graph.cypher().transaction()
        .with_statement(statement)
        .begin().unwrap()
        .0.commit().unwrap();

    let results = graph.cypher()
        .exec("MATCH (n:NTLY_INTG_TEST_2) RETURN n")
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    graph.cypher().exec("MATCH (n:NTLY_INTG_TEST_2) DELETE n").unwrap();
}

#[test]
fn transaction_create_after_begin_commit() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI).unwrap();
    let (mut transaction, _) = graph.cypher().transaction().begin().unwrap();

    let statement = Statement::new(
        "CREATE (n:NTLY_INTG_TEST_3 {lang})")
        .with_param("lang", &rust);

    transaction.exec(statement).unwrap();
    transaction.commit().unwrap();

    let results = graph.cypher()
        .exec("MATCH (n:NTLY_INTG_TEST_3) RETURN n")
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    graph.cypher().exec("MATCH (n:NTLY_INTG_TEST_3) DELETE n").unwrap();
}

#[test]
fn transaction_create_on_commit() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:NTLY_INTG_TEST_4 {lang})")
        .with_param("lang", &rust);

    let (mut transaction, _) = graph.cypher().transaction().begin().unwrap();
    transaction.add_statement(statement);
    transaction.commit().unwrap();

    let results = graph.cypher()
        .exec("MATCH (n:NTLY_INTG_TEST_4) RETURN n")
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    graph.cypher().exec("MATCH (n:NTLY_INTG_TEST_4) DELETE n").unwrap();
}

#[test]
fn transaction_create_on_begin_rollback() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:NTLY_INTG_TEST_5 {lang})")
        .with_param("lang", &rust);

    let (mut transaction, _) = graph.cypher().transaction()
        .with_statement(statement)
        .begin().unwrap();

    let results = transaction
        .exec("MATCH (n:NTLY_INTG_TEST_5) RETURN n")
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    transaction.rollback().unwrap();

    let results = graph.cypher()
        .exec("MATCH (n:NTLY_INTG_TEST_5) RETURN n")
        .unwrap();

    assert_eq!(0, results.rows().count());
}

#[test]
fn transaction_create_after_begin_rollback() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:NTLY_INTG_TEST_6 {lang})")
        .with_param("lang", &rust);

    let (mut transaction, _) = graph.cypher().transaction().begin().unwrap();
    transaction.exec(statement).unwrap();

    let results = transaction
        .exec("MATCH (n:NTLY_INTG_TEST_6) RETURN n")
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    transaction.rollback().unwrap();

    let results = graph.cypher()
        .exec("MATCH (n:NTLY_INTG_TEST_6) RETURN n")
        .unwrap();

    assert_eq!(0, results.rows().count());
}
