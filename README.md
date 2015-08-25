# rusted-cypher
Rust crate for accessing the cypher endpoint of a neo4j server

This is a prototype for accessing the cypher endpoint of a neo4j server, like a sql
driver for a relational database.

The main goal of this project is to provide a way to send cypher queries to a neo4j server and retrieve the results.
The second goal is to manage transactions through the transaction endpoint.

It MAY be extended to support other resources of the neo4j REST api.

At this moment, it is only possible to send one or more cypher queries, closing the transaction immediatly.
Managing an open transaction still needs to be done, but eventually will.

## Examples

```rust
extern crate rusted_cypher;

use std::collections::BTreeMap;
use rusted_cypher::GraphClient;
use rusted_cypher::cypher::Statement;

fn main() {
    let graph = GraphClient::connect("http://neo4j:neo4j@localhost:7474/db/data").unwrap();

    let mut query = graph.cypher().query();
    query.add_simple_statement("CREATE (n:LANG { name: 'Rust', level: 'low', safe: true })");

    let mut params = BTreeMap::new();
    params.insert("safeness", false);
    query.add_statement(Statement::new("CREATE (n:LANG { name: 'C++', level: 'low', safe: {safeness} })", &params));

    query.send().unwrap();

    graph.cypher().exec("CREATE (n:LANG { name: 'Python', level: 'High', safe: true })").unwrap();

    let result = graph.cypher().exec("MATCH (n:LANG) RETURN n").unwrap();

    for row in result.iter() {
        println!("{:?}", row);
    }

    graph.cypher().exec("MATCH (n:LANG) DELETE n").unwrap();
}
```
