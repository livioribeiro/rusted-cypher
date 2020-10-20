extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate rusted_cypher;

use rusted_cypher::GraphClient;
use rusted_cypher::cypher::result::Row;

const URI: &'static str = "http://neo4j:neo4j@127.0.0.1:7474/db/data";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Language {
    name: String,
    level: String,
    safe: bool,
}

#[tokio::test]
async fn without_params() {
    let graph = GraphClient::connect(URI, None).await.unwrap();

    let stmt = cypher_stmt!("MATCH (n:NTLY_INTG_TEST_MACROS_1) RETURN n").unwrap();

    let result = graph.exec(stmt).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn save_retrive_struct() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let graph = GraphClient::connect(URI, None).await.unwrap();

    let stmt = cypher_stmt!("CREATE (n:NTLY_INTG_TEST_MACROS_2 $lang) RETURN n", {
        "lang" => &rust
    }).unwrap();

    let results = graph.exec(stmt).await.unwrap();
    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let lang: Language = row.get("n").unwrap();

    assert_eq!(rust, lang);

    graph.exec("MATCH (n:NTLY_INTG_TEST_MACROS_2) DELETE n").await.unwrap();
}
