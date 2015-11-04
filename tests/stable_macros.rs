extern crate serde;

#[macro_use]
extern crate rusted_cypher;

use rusted_cypher::GraphClient;
use rusted_cypher::cypher::result::Row;

const URI: &'static str = "http://neo4j:neo4j@127.0.0.1:7474/db/data";

#[test]
fn without_params() {
    let graph = GraphClient::connect(URI).unwrap();

    let stmt = cypher_stmt!("MATCH (n:INTG_TEST_MACROS_1) RETURN n");

    let result = graph.cypher().exec(stmt);
    assert!(result.is_ok());
}

#[test]
fn save_retrive_values() {
    let graph = GraphClient::connect(URI).unwrap();

    let stmt = cypher_stmt!(
        "CREATE (n:INTG_TEST_MACROS_2 {name: {name}, level: {level}, safe: {safe}}) RETURN n.name, n.level, n.safe" {
            "name" => "Rust",
            "level" => "low",
            "safe" => true
        }
    );

    let results = graph.cypher().exec(stmt).unwrap();

    let rows: Vec<Row> = results.rows().take(1).collect();
    let row = rows.first().unwrap();

    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safe: bool = row.get("n.safe").unwrap();

    assert_eq!("Rust", name);
    assert_eq!("low", level);
    assert_eq!(true, safe);

    graph.cypher().exec("MATCH (n:INTG_TEST_MACROS_2) DELETE n").unwrap();
}
