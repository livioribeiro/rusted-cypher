# rusted-cypher
Rust crate for accessing the cypher endpoint of a neo4j server

This crate is a prototype for a client for the cypher endpoint of a neo4j server, like a sql
driver for a relational database.

The goal of this project is to provide a way to send cypher queries to a neo4j server and iterate over the results.
It MAY be extended to support other resources of the neo4j REST api.

At this moment, it is only possible to send one or more cypher queries, closing the transaction immediatly.
Managing an open transaction still needs to be done, but eventually will.

## Examples

```rust
extern crate rusted_cypher;

use rusted_cypher::GraphClient;

fn main() {
    let graph = GraphClient::connect(
        "http://neo4j:neo4j@localhost:7474/db/data"
    ).unwrap();

    let result = graph.cypher_query("match n return n").unwrap();

    for cypher_result in result {
        println!("{:?}", cypher_result.data);
    }
}
```
