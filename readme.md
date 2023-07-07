# Postgres Native Data Connector

## Build

### Prequisites

1. Install [rustup](https://www.rust-lang.org/tools/install).
2. Install additional tools:
    - `cargo install watch insta`
    - `rustup component add rust-analyzer`
    - `rustup component add clippy`
    - `rustup component add rustfmt`
3. Install [just](https://github.com/casey/just)
4. Install [docker](https://www.docker.com/)
5. Install protoc. Here are a few options:
    - `brew install protobuf`
    - `apt-get install protobuf-compiler`
    - `dnf install protobuf-compiler`
6. Clone v3 experiments in a directory near this one: `(cd .. && git clone git@github.com:hasura/v3-experiments.git)`

### Compile

```
cargo build
```

### Run

Run the multitenant Postgres agent with:

```
just run-postgres-ndc
```

### Develop

1. Start the database sample: `just start-docker`
2. Compile, run tests, and rerun server on file changes: `just dev`
3. Query the connector via curl: `curl -X POST http://localhost:8666/query`

## General structure

The turn a request into a query and run it work lives in [crates/query-engine/](crates/query-engine/).
- Input and output types imported from: [crates/gdc-client/src/models.rs](crates/gdc-client/src/models.rs)
- Compiler phases: [src/phases/](crates/query-engine/src/phases/):
   - Translation from query request to sql ast: [src/phases/translation.rs](crates/query-engine/src/phases/translation.rs)
   - Translation from sql_ast to sql_string: [src/phases/translation/](crates/query-engine/src/phases/translation/)
   - Execution of the plan against postgres: [src/phases/execution.rs](crates/query-engine/src/phases/execution.rs)
- Unit and integration tests: [tests/](crates/query-engine/)

The multitenant server lives in [crates/postgres-multitenant-ndc](crates/postgres-multitenant-ndc).
- Entry point: [src/main.rs](crates/postgres-multitenant-ndc/src/main.rs)
- Routing: [src/routes/](crates/postgres-multitenant-ndc/src/routes/mod.rs)

## Example

1. Run `just dev`
2. Run `just run-v3`
3. Connect to GraphiQL at http://localhost:3000 and run a query:

   ```graphql
   query {
     AlbumByID(AlbumId: 35) {
       Title
     }
   }
   ```

## Multitenant example

1. Run `just run-postgres-multitenant-dc`
2. Run `just run-v3-multitenant`
3. Run `just test-multitenant`

## Write a test

1. Create a new file under `crates/postgres-multitenant-ndc/tests/goldenfiles/<your-test-name>.json`
2. Create a new test in `crates/postgres-multitenant-ndc/tests/tests.rs` that looks like this:
   ```rs
   #[tokio::test]
   async fn select_5() {
       let result = common::test_query("select_5").await;
       insta::assert_json_snapshot!(result);
   }
   ```
3. Run the tests using `just dev`
4. Review the results using `cargo insta review`

## Linting

Run `just lint` to run clippy linter

run `just lint-apply` to attempt to autofix all linter suggestions

## Formatting

Check your formatting is great with `just format-check`.

Format all Rust code with `just format`.
