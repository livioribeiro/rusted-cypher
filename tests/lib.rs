extern crate serde;
extern crate rusted_cypher;

use rusted_cypher::{GraphClient, Statement};
use rusted_cypher::cypher::result::Row;

const URI: &'static str = "http://neo4j:neo4j@127.0.0.1:7474/db/data";

#[test]
fn save_retrive_values() {
    let client = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_1 {name: {name}, level: {level}, safe: {safe}}) RETURN n.name, n.level, n.safe")
        .with_param("name", "Rust")
        .with_param("level", "low")
        .with_param("safe", true);

    let results = client.cypher().exec(statement).unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    client.cypher().exec("MATCH (n:INTG_TEST_1) DELETE n").unwrap();
}

#[test]
fn transaction_create_on_begin_commit() {
    let client = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_2 {name: {name}, level: {level}, safe: {safe}})")
        .with_param("name", "Rust")
        .with_param("level", "low")
        .with_param("safe", true);

    client.cypher().transaction()
        .with_statement(statement)
        .begin().unwrap()
        .0.commit().unwrap();

    let results = client.cypher()
        .exec("MATCH (n:INTG_TEST_2) RETURN n.name, n.level, n.safe")
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    client.cypher().exec("MATCH (n:INTG_TEST_2) DELETE n").unwrap();
}

#[test]
fn transaction_create_after_begin_commit() {
    let client = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_3 {name: {name}, level: {level}, safe: {safe}})")
        .with_param("name", "Rust")
        .with_param("level", "low")
        .with_param("safe", true);

    let (mut transaction, _) = client.cypher().transaction().begin().unwrap();
    transaction.add_statement(statement);
    transaction.exec().unwrap();
    transaction.commit().unwrap();

    let results = client.cypher()
        .exec("MATCH (n:INTG_TEST_3) RETURN n.name, n.level, n.safe")
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    client.cypher().exec("MATCH (n:INTG_TEST_3) DELETE n").unwrap();
}

#[test]
fn transaction_create_on_commit() {
    let client = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_4 {name: {name}, level: {level}, safe: {safe}})")
        .with_param("name", "Rust")
        .with_param("level", "low")
        .with_param("safe", true);

    let (mut transaction, _) = client.cypher().transaction().begin().unwrap();
    transaction.add_statement(statement);
    transaction.commit().unwrap();

    let results = client.cypher()
        .exec("MATCH (n:INTG_TEST_4) RETURN n.name, n.level, n.safe")
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    client.cypher().exec("MATCH (n:INTG_TEST_4) DELETE n").unwrap();
}

#[test]
fn transaction_create_on_begin_rollback() {
    let client = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_5 {name: {name}, level: {level}, safe: {safe}})")
        .with_param("name", "Rust")
        .with_param("level", "low")
        .with_param("safe", true);

    let (mut transaction, _) = client.cypher().transaction()
        .with_statement(statement)
        .begin().unwrap();

    let results = transaction
        .with_statement("MATCH (n:INTG_TEST_5) RETURN n.name, n.level, n.safe")
        .exec()
        .unwrap();

    let rows: Vec<Row> = results[0].rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    transaction.rollback().unwrap();

    let results = client.cypher()
        .exec("MATCH (n:INTG_TEST_5) RETURN n")
        .unwrap();

    assert_eq!(0, results.rows().count());
}

#[test]
fn transaction_create_after_begin_rollback() {
    let client = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_6 {name: {name}, level: {level}, safe: {safe}})")
        .with_param("name", "Rust")
        .with_param("level", "low")
        .with_param("safe", true);

    let (mut transaction, _) = client.cypher().transaction().begin().unwrap();
    transaction.add_statement(statement);
    transaction.exec().unwrap();

    let results = transaction
        .with_statement("MATCH (n:INTG_TEST_6) RETURN n.name, n.level, n.safe")
        .exec()
        .unwrap();

    let rows: Vec<Row> = results[0].rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    transaction.rollback().unwrap();

    let results = client.cypher()
        .exec("MATCH (n:INTG_TEST_6) RETURN n")
        .unwrap();

    assert_eq!(0, results.rows().count());
}
