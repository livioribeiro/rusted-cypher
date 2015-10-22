#![cfg_attr(feature = "serde_macros", feature(custom_derive, plugin))]
#![cfg_attr(feature = "serde_macros", plugin(serde_macros))]

extern crate serde;
extern crate rusted_cypher;

use rusted_cypher::{GraphClient, Statement};

const URI: &'static str = "http://neo4j:neo4j@127.0.0.1:7474/db/data";


#[cfg(feature = "serde_macros")]
#[derive(Serialize, Deserialize)]
struct Language {
    name: String,
    level: String,
    safe: bool,
}

#[cfg(feature = "serde_macros")]
#[test]
fn save_retrieve_struct() {
    let rust = Language {
        name: "Rust".to_owned(),
        level: "low".to_owned(),
        safe: true,
    };

    let client = GraphClient::connect(URI).unwrap();
    let statement = Statement::new("CREATE (n:INTG_TEST_1 {lang}) RETURN n")
        .with_param("lang", &rust);

    let results = client.cypher().exec(statement).unwrap();

    for row in results[0].rows() {
        let lang: Language = row.get("n").unwrap();

        assert_eq!("Rust", lang.name);
        assert_eq!("low", lang.level);
        assert_eq!(true, lang.safe);
    }

    client.cypher().exec("MATCH (n:INTG_TEST_1) DELETE n").unwrap();
}

#[test]
fn save_retrive_values() {
    let client = GraphClient::connect(URI).unwrap();
    let statement = Statement::new(
        "CREATE (n:INTG_TEST_2 {name: {name}, level: {level}, safe: {safe}}) RETURN n.name, n.level, n.safe")
        .with_param("name", "Rust")
        .with_param("level", "low")
        .with_param("safe", true);

    let results = client.cypher().exec(statement).unwrap();

    for row in results[0].rows() {
        let name: String = row.get("n.name").unwrap();
        let level: String = row.get("n.level").unwrap();
        let safe: bool = row.get("n.safe").unwrap();

        assert_eq!("Rust", name);
        assert_eq!("low", level);
        assert_eq!(true, safe);
    }

    client.cypher().exec("MATCH (n:INTG_TEST_2) DELETE n").unwrap();
}
