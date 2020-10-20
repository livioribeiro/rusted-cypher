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

#[tokio::test]
async fn save_retrieve_struct() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };
    
    let graph = GraphClient::connect(URI, None).await.unwrap();

    let statement = Statement::new("CREATE (n:NTLY_INTG_TEST_1 $lang) RETURN n")
        .with_param("lang", &rust).unwrap();

    let results = graph.exec(statement).await.unwrap();
    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    graph.exec("MATCH (n:NTLY_INTG_TEST_1) DELETE n").await.unwrap();
}

#[tokio::test]
async fn transaction_create_on_begin_commit() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI, None).await.unwrap();

    let statement = Statement::new(
        "CREATE (n:NTLY_INTG_TEST_2 $lang)")
        .with_param("lang", &rust).unwrap();

    graph.transaction()
        .with_statement(statement)
        .begin().await.unwrap()
        .0.commit().await.unwrap();

    let results = graph.exec("MATCH (n:NTLY_INTG_TEST_2) RETURN n")
        .await
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    graph.exec("MATCH (n:NTLY_INTG_TEST_2) DELETE n").await.unwrap();
}

#[tokio::test]
async fn transaction_create_after_begin_commit() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI, None).await.unwrap();
    let (mut transaction, _) = graph.transaction().begin().await.unwrap();

    let statement = Statement::new(
        "CREATE (n:NTLY_INTG_TEST_3 $lang)")
        .with_param("lang", &rust).unwrap();

    transaction.exec(statement).await.unwrap();
    transaction.commit().await.unwrap();

    let results = graph.exec("MATCH (n:NTLY_INTG_TEST_3) RETURN n")
        .await
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    graph.exec("MATCH (n:NTLY_INTG_TEST_3) DELETE n").await.unwrap();
}

#[tokio::test]
async fn transaction_create_on_commit() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI, None).await.unwrap();

    let statement = Statement::new(
        "CREATE (n:NTLY_INTG_TEST_4 $lang)")
        .with_param("lang", &rust).unwrap();

    let (mut transaction, _) = graph.transaction().begin().await.unwrap();
    transaction.add_statement(statement);
    transaction.commit().await.unwrap();

    let results = graph
        .exec("MATCH (n:NTLY_INTG_TEST_4) RETURN n")
        .await
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    graph.exec("MATCH (n:NTLY_INTG_TEST_4) DELETE n").await.unwrap();
}

#[tokio::test]
async fn transaction_create_on_begin_rollback() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI, None).await.unwrap();

    let statement = Statement::new(
        "CREATE (n:NTLY_INTG_TEST_5 $lang)")
        .with_param("lang", &rust).unwrap();

    let (mut transaction, _) = graph.transaction()
        .with_statement(statement)
        .begin().await.unwrap();

    let results = transaction
        .exec("MATCH (n:NTLY_INTG_TEST_5) RETURN n")
        .await
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    transaction.rollback().await.unwrap();

    let results = graph.exec("MATCH (n:NTLY_INTG_TEST_5) RETURN n")
        .await
        .unwrap();

    assert_eq!(0, results.rows().count());
}

#[tokio::test]
async fn transaction_create_after_begin_rollback() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI, None).await.unwrap();

    let statement = Statement::new(
        "CREATE (n:NTLY_INTG_TEST_6 $lang)")
        .with_param("lang", &rust).unwrap();

    let (mut transaction, _) = graph.transaction().begin().await.unwrap();
    transaction.exec(statement).await.unwrap();

    let results = transaction
        .exec("MATCH (n:NTLY_INTG_TEST_6) RETURN n")
        .await
        .unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    transaction.rollback().await.unwrap();

    let results = graph.exec("MATCH (n:NTLY_INTG_TEST_6) RETURN n")
        .await
        .unwrap();

    assert_eq!(0, results.rows().count());
}
