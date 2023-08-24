# Postgres Native Data Connector

## Build

### Prequisites

1. Install [rustup](https://www.rust-lang.org/tools/install).
2. Install additional tools:
    - `cargo install cargo-watch cargo-insta`
3. Install [just](https://github.com/casey/just)
4. Install [Docker](https://www.docker.com/)
5. Install protoc. Here are a few options:
    - `brew install protobuf`
    - `apt-get install protobuf-compiler`
    - `dnf install protobuf-compiler`
6. Clone [v3-engine](https://github.com/hasura/v3-engine) in a directory near this one:
   ```
   (cd .. && git clone git@github.com:hasura/v3-engine.git)
   ```

### Compile

```
cargo build
```

### Run

Run the postgres agent with:

```
just run
```

### Develop

1. Start the sample chinook postgres db, compile, run tests, and rerun server on file changes: `just dev`
2. Query the connector via curl:
   ```
   curl -H "Content-Type: application/json" \
     --data "@crates/ndc-postgres/tests/goldenfiles/select_where_variable.json" \
	 http://localhost:8100/query \
	 | jq
   ```

Among the docker containers is a Jaeger instance for tracing/debugging, accessible at http://127.0.0.1:4002.

## Debug

See [debugging.md](./debugging.md).

### Profile

We can produce a flamegraph using `just flamegraph` using [flamegraph-rs](https://github.com/flamegraph-rs/flamegraph). Follow the installation instructions.

### Benchmark

See [./benchmarks/component/README.md](./benchmarks/component/README.md).

A benchmark history can be viewed [here](https://hasura.github.io/postgres-ndc/dev/bench).

## General structure

See [architecture.md](./architecture.md).

## Example

1. Run `just dev` (or `just run`)
2. Run `just run-engine`
3. Connect to GraphiQL at http://localhost:3000 and run a query:

   ```graphql
   query {
     AlbumByID(AlbumId: 35) {
       Title
     }
   }
   ```
   (or `just test-integrated`)

## Write a database execution test

1. Create a new file under `crates/ndc-postgres/tests/goldenfiles/<your-test-name>.json`
2. Create a new test in `crates/ndc-postgres/tests/tests.rs` that looks like this:
   ```rs
   #[tokio::test]
   async fn select_5() {
       let result = common::test_query("select_5").await;
       insta::assert_json_snapshot!(result);
   }
   ```
3. Run the tests using `just dev`
4. Review the results using `cargo insta review`

## Write a SQL translation snapshot test

1. Create a new folder under `crates/query-engine/tests/goldenfiles/<your-test-name>/`
2. Create `request.json` and `tables.json` files in that folder to specify your
   request
3. Create a new test in `crates/query-engine/tests/tests.rs` that looks like this:
   ```rs
   #[tokio::test]
   async fn select_5() {
       let result = common::test_translation("select_5").await;
       insta::assert_snapshot!(result);
   }
   ```
4. Run the tests using `just dev`
5. Review the results using `cargo insta review`

## Testing metrics

We have a Prometheus / Grafana set up in Docker. Run `just start-metrics` to
start them, you can then navigation to `localhost:3001` for Grafana, or
`localhost:9090` for Prometheus.

## Linting

Run `just lint` to run clippy linter

run `just lint-apply` to attempt to autofix all linter suggestions

## Formatting

Check your formatting is great with `just format-check`.

Format all Rust code with `just format`.
