# rusted-cypher
Rust crate for accessing the cypher endpoint of a neo4j server

This is a prototype for accessing the cypher endpoint of a neo4j server, like a sql
driver for a relational database.

You can execute queries inside a transaction or simply execute queries that commit immediately.

It MAY be extended to support other resources of the neo4j REST api.

## Examples

```rust
extern crate rusted_cypher;

use std::collections::BTreeMap;
use rusted_cypher::GraphClient;
use rusted_cypher::cypher::Statement;

fn main() {
    let graph = GraphClient::connect(
        "http://neo4j:neo4j@localhost:7474/db/data").unwrap();

    // Without transactions
    let mut query = graph.cypher().query();
    query.add_simple_statement(
        "CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })");

    let mut params = BTreeMap::new();
    params.insert("safeness", false);
    query.add_statement((
        "CREATE (n:LANG { name: 'C++', level: 'low', safe: {safeness} })",
         &params
    ));

    query.send().unwrap();

    graph.cypher().exec(
        "CREATE (n:LANG { name: 'Python', level: 'high', safe: true })"
    ).unwrap();

    let result = graph.cypher().exec("MATCH (n:LANG) RETURN n").unwrap();

    for row in result.iter() {
        println!("{:?}", row);
    }

    graph.cypher().exec("MATCH (n:LANG) DELETE n").unwrap();

    // With transactions
    let params: BTreeMap<String, String> = BTreeMap::new();
    let stmt = Statement::new(
        "CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })",
        &params
    );

    let (mut transaction, results)
        = graph.cypher().begin_transaction(vec![stmt]).unwrap();

    let stmt = Statement::new(
        "CREATE (n:LANG { name: 'Python', level: 'high', safe: true })",
        &params
    );

    transaction.exec(vec![stmt]).unwrap();

    let mut params = BTreeMap::new();
    params.insert("safeness", true);

    let stmt = Statement::new(
        "MATCH (n:LANG) WHERE (n.safe = {safeness}) RETURN n",
        &params
    );
    let results = transaction.exec(vec![stmt]).unwrap();

    assert_eq!(results[0].data.len(), 2);

    transaction.rollback();
}
```
