# 0.4.1

  - Updated `serde_macros` and `serde_codegen` to version 0.6.
  - Added `log` support.

# 0.4

  - Refactored `cypher` module. Code from `cypher.rs` is now at `cypher/mod.rs`.
  - Added `CypherResult::rows` to iterate over results.
  - Added builder pattern for creating statements with parameters.
  - Added builder pattern for creating transaction with statements.

# 0.3

  - Refactored of `statement` module.
  - Implemented `From<&str> for Statement`.

# 0.2

  - Added `transaction` module.
