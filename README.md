# rusted-cypher
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

let mut params = BTreeMap::new();
params.insert("safeness", false);

// Statement also implements From<(&str, &BTreeMap<String, Value)>
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
```

### With Transactions

```rust
let stmt = Statement::from(
    "CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })");

let (mut transaction, results)
    = graph.cypher().begin_transaction(vec![stmt]).unwrap();

let stmt = Statement::from(
    "CREATE (n:LANG { name: 'Python', level: 'high', safe: true })");

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
```
