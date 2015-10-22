# rusted_cypher

Rust crate for accessing the cypher endpoint of a neo4j server

This is a prototype for accessing the cypher endpoint of a neo4j server, like a sql
driver for a relational database.

You can execute queries inside a transaction or simply execute queries that commit immediately.

It MAY be extended to support other resources of the neo4j REST api.

## Examples

Code in examples are assumed to be wrapped in:

```rust
extern crate rusted_cypher;

use std::collections::BTreeMap;
use rusted_cypher::GraphClient;
use rusted_cypher::cypher::Statement;

fn main() {
  // Connect to the database
  let graph = GraphClient::connect(
      "http://neo4j:neo4j@localhost:7474/db/data").unwrap();

  // Example code here!
}
```

### Performing Queries

```rust
let mut query = graph.cypher().query();

// Statement implements From<&str>
query.add_statement(
    "CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })");

let statement = Statement::new(
    "CREATE (n:LANG { name: 'C++', level: 'low', safe: {safeness} })")
    .with_param("safeness", false);

query.add_statement(statement);

query.send().unwrap();

graph.cypher().exec(
    "CREATE (n:LANG { name: 'Python', level: 'high', safe: true })")
    .unwrap();

let result = graph.cypher().exec(
    "MATCH (n:LANG) RETURN n.name, n.level, n.safe")
    .unwrap();

assert_eq!(result[0].data.len(), 3);

for row in result[0].rows() {
    let name: String = row.get("n.name").unwrap();
    let level: String = row.get("n.level").unwrap();
    let safeness: bool = row.get("n.safe").unwrap();
    println!("name: {}, level: {}, safe: {}", name, level, safeness);
}

graph.cypher().exec("MATCH (n:LANG) DELETE n").unwrap();
```

### With Transactions

```rust
let transaction = graph.cypher().transaction()
    .with_statement("CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })");

let (mut transaction, results) = transaction.begin().unwrap();

transaction.add_statement("CREATE (n:LANG { name: 'Python', level: 'high', safe: true })");
transaction.exec().unwrap();

let stmt = Statement::new("MATCH (n:LANG) WHERE (n.safe = {safeness}) RETURN n")
    .with_param("safeness", true);

transaction.add_statement(stmt);
let results = transaction.exec().unwrap();

assert_eq!(results[0].data.len(), 2);

transaction.rollback();
```

License: MIT
