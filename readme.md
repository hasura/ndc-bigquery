# Postgres Native Data Connector

## Build

### Prequisites

1. Install [rustup](https://www.rust-lang.org/tools/install).
2. Install additional tools:
    - `cargo install watch cargo-insta`
    - `rustup component add rust-analyzer`
    - `rustup component add clippy`
    - `rustup component add rustfmt`
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
just run-postgres-ndc
```

### Develop

1. Start the database sample: `just start-docker`
2. Compile, run tests, and rerun server on file changes: `just dev`
3. Query the connector via curl:
   ```
   curl -H "Content-Type: application/json" \
     --data "@crates/ndc-postgres/tests/goldenfiles/select_where_variable.json" \
	 http://localhost:8100/query \
	 | jq
   ```

Among the docker containers is a Jaeger instance for tracing/debugging, accessible at http://127.0.0.1:4002.

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

*warning* - SQL translation tests should aim to only contain one field or aggregate per
request. This is because we parse the fields into a HashMap and cannot rely on
the ordering, meaning the snapshots constantly change.

## Linting

Run `just lint` to run clippy linter

run `just lint-apply` to attempt to autofix all linter suggestions

## Formatting

Check your formatting is great with `just format-check`.

Format all Rust code with `just format`.

## Configuring OpenTelemetry Tracing

Traces in the agent program that are emitted using the `tracing` crate are
automatically exported using OpenTelemetry's OTLP. HTTP request handlers are
automatically instrumented. We also propagate parent trace info from incoming
HTTP requests.

Traces are printed to stdout, and exported via OTLP. To configure exporting set
these environment variables:

- `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` - defaults to `http://localhost:4318`
- `OTEL_EXPORTER_OTLP_TRACES_PROTOCOL` - optional, may be `grpc` or `http/protobuf`; see note below
- `OTEL_SERVICE_NAME` - name of the service
- `OTEL_PROPAGATORS` - comma-separate list of propagator schemes to use; may be
  - `b3`
  - `b3multi`
  - `tracecontext` (default)
  - `baggage`
  - `none`
- `OTEL_TRACES_SAMPLER` - select a sampler; may be
  - `always_on`
  - `always_off`
  - `traceidratio`
  - `parentbased_always_on` (default)
  - `parentbased_always_off`
  - `parentbased_traceidratio`
- `OTEL_TRACES_SAMPLER_ARG` - additional configuration of the sampler
- `OTEL_LOG` or `RUST_LOG` - set logging level thresholds; defaults to `info`; search for docs on `RUST_LOG`

If you don't specify an OTLP protocol then one is inferred. If the endpoint uses
the port `4317` then the protocol is inferred to be `grpc`. Otherwise
`http/protobuf` is used.

Internally logging levels are set like this:

    "$OTEL_LOG,axum_tracing_opentelemetry=info,otel=debug"

So traces from `axum_tracing_opentelemetry` default to `info`, logs from `otel`
default to `debug`, and everything else defaults to `info`.
