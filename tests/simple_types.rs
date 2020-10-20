extern crate rusted_cypher;

use rusted_cypher::{GraphClient, Statement};
use rusted_cypher::cypher::result::Row;

const URI: &'static str = "http://neo4j:neo4j@127.0.0.1:7474/db/data";

#[tokio::test]
async fn save_retrive_values() {
    let graph = GraphClient::connect(URI, None).await.unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_1 {name: $name, level: $level, safe: $safe}) RETURN n.name, n.level, n.safe")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    let results = graph.exec(statement).await.unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    graph.exec("MATCH (n:INTG_TEST_1) DELETE n").await.unwrap();
}

#[tokio::test]
async fn transaction_create_on_begin_commit() {
    let graph = GraphClient::connect(URI, None).await.unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_2 {name: $name, level: $level, safe: $safe})")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    graph.transaction()
        .with_statement(statement)
        .begin().await.unwrap()
        .0.commit().await.unwrap();

    let results = graph.exec("MATCH (n:INTG_TEST_2) RETURN n.name, n.level, n.safe")
        .await
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    graph.exec("MATCH (n:INTG_TEST_2) DELETE n").await.unwrap();
}

#[tokio::test]
async fn transaction_create_after_begin_commit() {
    let graph = GraphClient::connect(URI, None).await.unwrap();
    let (mut transaction, _) = graph.transaction().begin().await.unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_3 {name: $name, level: $level, safe: $safe})")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    transaction.exec(statement).await.unwrap();
    transaction.commit().await.unwrap();

    let results = graph.exec("MATCH (n:INTG_TEST_3) RETURN n.name, n.level, n.safe")
        .await
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    graph.exec("MATCH (n:INTG_TEST_3) DELETE n").await.unwrap();
}

#[tokio::test]
async fn transaction_create_on_commit() {
    let graph = GraphClient::connect(URI, None).await.unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_4 {name: $name, level: $level, safe: $safe})")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    let (mut transaction, _) = graph.transaction().begin().await.unwrap();
    transaction.add_statement(statement);
    transaction.commit().await.unwrap();

    let results = graph.exec("MATCH (n:INTG_TEST_4) RETURN n.name, n.level, n.safe")
        .await
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    graph.exec("MATCH (n:INTG_TEST_4) DELETE n").await.unwrap();
}

#[tokio::test]
async fn transaction_create_on_begin_rollback() {
    let graph = GraphClient::connect(URI, None).await.unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_5 {name: $name, level: $level, safe: $safe})")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    let (mut transaction, _) = graph.transaction()
        .with_statement(statement)
        .begin().await.unwrap();

    let result = transaction
        .exec("MATCH (n:INTG_TEST_5) RETURN n.name, n.level, n.safe")
        .await
        .unwrap();

    let rows: Vec<Row> = result.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    transaction.rollback().await.unwrap();

    let results = graph.exec("MATCH (n:INTG_TEST_5) RETURN n")
        .await
        .unwrap();

    assert_eq!(0, results.rows().count());
}

#[tokio::test]
async fn transaction_create_after_begin_rollback() {
    let graph = GraphClient::connect(URI, None).await.unwrap();
    let (mut transaction, _) = graph.transaction().begin().await.unwrap();

    let statement = Statement::new(
        "CREATE (n:INTG_TEST_6 {name: $name, level: $level, safe: $safe})")
        .with_param("name", "Rust").unwrap()
        .with_param("level", "low").unwrap()
        .with_param("safe", true).unwrap();

    transaction.exec(statement).await.unwrap();

    let results = transaction
        .exec("MATCH (n:INTG_TEST_6) RETURN n.name, n.level, n.safe")
        .await
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    transaction.rollback().await.unwrap();

    let results = graph.exec("MATCH (n:INTG_TEST_6) RETURN n")
        .await
        .unwrap();

    assert_eq!(0, results.rows().count());
}
