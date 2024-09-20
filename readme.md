# ndc-bigquery

BigQuery NDC.

Forked from `ndc-postgres`.

## Getting started

1. Set up `cargo` and friends for Rust development as per instructions in [ndc-postgres](https://github.com/hasura/ndc-postgres).
2. Get a BigQuery service account key and put it in the
   `HASURA_BIGQUERY_SERVICE_KEY`, `HASURA_BIGQUERY_PROJECT_ID` and `HASURA_BIGQUERY_DATASET_ID` env var
3. Create configuration file using the [CLI instructions](./crates/cli/readme.md)
4. Run `cargo run --bin ndc-bigquery -- serve --configuration "<configuration-directory>/"`
