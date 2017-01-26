extern crate rusted_cypher;

use rusted_cypher::{GraphClient, Statement};
use rusted_cypher::cypher::result::Row;

const URI: &'static str = "http://neo4j:neo4j@127.0.0.1:7474/db/data";

#[test]
fn save_retrive_values() {
    let graph = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_1 {name: {name}, level: {level}, safe: {safe}}) RETURN n.name, n.level, n.safe")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    let results = graph.cypher().exec(statement).unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    graph.cypher().exec("MATCH (n:INTG_TEST_1) DELETE n").unwrap();
}

#[test]
fn transaction_create_on_begin_commit() {
    let graph = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_2 {name: {name}, level: {level}, safe: {safe}})")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    graph.cypher().transaction()
        .with_statement(statement)
        .begin().unwrap()
        .0.commit().unwrap();

    let results = graph.cypher()
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

    graph.cypher().exec("MATCH (n:INTG_TEST_2) DELETE n").unwrap();
}

#[test]
fn transaction_create_after_begin_commit() {
    let graph = GraphClient::connect(URI).unwrap();
    let (mut transaction, _) = graph.cypher().transaction().begin().unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_3 {name: {name}, level: {level}, safe: {safe}})")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    transaction.exec(statement).unwrap();
    transaction.commit().unwrap();

    let results = graph.cypher()
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

    graph.cypher().exec("MATCH (n:INTG_TEST_3) DELETE n").unwrap();
}

#[test]
fn transaction_create_on_commit() {
    let graph = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_4 {name: {name}, level: {level}, safe: {safe}})")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    let (mut transaction, _) = graph.cypher().transaction().begin().unwrap();
    transaction.add_statement(statement);
    transaction.commit().unwrap();

    let results = graph.cypher()
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

    graph.cypher().exec("MATCH (n:INTG_TEST_4) DELETE n").unwrap();
}

#[test]
fn transaction_create_on_begin_rollback() {
    let graph = GraphClient::connect(URI).unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_5 {name: {name}, level: {level}, safe: {safe}})")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    let (mut transaction, _) = graph.cypher().transaction()
        .with_statement(statement)
        .begin().unwrap();

    let result = transaction
        .exec("MATCH (n:INTG_TEST_5) RETURN n.name, n.level, n.safe")
        .unwrap();

    let rows: Vec<Row> = result.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    transaction.rollback().unwrap();

    let results = graph.cypher()
        .exec("MATCH (n:INTG_TEST_5) RETURN n")
        .unwrap();

    assert_eq!(0, results.rows().count());
}

#[test]
fn transaction_create_after_begin_rollback() {
    let graph = GraphClient::connect(URI).unwrap();
    let (mut transaction, _) = graph.cypher().transaction().begin().unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_6 {name: {name}, level: {level}, safe: {safe}})")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    transaction.exec(statement).unwrap();

    let results = transaction
        .exec("MATCH (n:INTG_TEST_6) RETURN n.name, n.level, n.safe")
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    transaction.rollback().unwrap();

    let results = graph.cypher()
        .exec("MATCH (n:INTG_TEST_6) RETURN n")
        .unwrap();

    assert_eq!(0, results.rows().count());
}
