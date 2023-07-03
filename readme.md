# Postgres Native Data Connector

## Build

### Prequisites

1. Install [rustup](https://www.rust-lang.org/tools/install).
2. Install additional tools:
    - `cargo install watch rustfmt insta`
    - `rustup component add rust-analyzer`
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

Run the main Postgres agent with:

```
just run-postgres-dc
```

Alternatively, run the multitenant Postgres agent with:

```
just run-postgres-multitenant-dc
```

### Develop

1. Start the database sample: `just start-docker`
2. Compile, run tests, and rerun server on file changes: `just dev`
3. Query the connector via curl: `curl -X POST http://localhost:8666/query`

## General structure

Our work is mainly in [crates/postgres-ndc/](crates/postgres-ndc/).

- Entry point: [src/main.rs](crates/postgres-ndc/src/main.rs)
- Connector state: [src/connector.rs](crates/postgres-ndc/src/connector.rs)
- Routing: [src/routes/](crates/postgres-ndc/src/routes.rs)
- Input and output types imported from: [crates/gdc-client/src/models.rs](crates/gdc-client/src/models.rs)
- Compiler phases: [src/phases/](crates/postgres-ndc/src/phases/):
   - Translation from query request to sql ast: [src/phases/translation.rs](crates/postgres-ndc/src/phases/translation.rs)
   - Translation from sql_ast to sql_string: [src/phases/translation/](crates/postgres-ndc/src/phases/translation/)
   - Execution of the plan against postgres: [src/phases/execution.rs](crates/postgres-ndc/src/phases/execution.rs)
- Unit and integration tests: [tests/](crates/postgres-ndc/)

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

1. Create a new file under `crates/postgres-ndc/tests/goldenfiles/<your-test-name>.json`
2. Create a new test in `crates/postgres-ndc/tests/tests.rs` that looks like this:
   ```rs
   #[tokio::test]
   async fn select_5() {
       let result = test_query("select_5").await;
       insta::assert_snapshot!(result);
   }
   ```
3. Run the tests using `just dev`
4. Review the results using `cargo insta review`
