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
6. Clone v3 experiments in a directory near this one: `(cd .. && git clone git@github.com:hasura/v3-experiments.git)`

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

1. Run `just run-postgres-ndc`
2. Run `just run-v3-multitenant`
3. Run `just test-multitenant`

## Write a test

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

## Deployment files

In the multitenant agent we use a number of `deployment` files to configure
each tenant. These are served from a folder (locally, we use
`./static/deployments`) so the data in `./static/deployments/123456.json` will
setup requests to `/deployment/123456/query`.

However, we can't just use the JSON files. They must be validated and then
turned into `.bin` files in the same folder. We can do this by starting the multitenant agent with `just
run-multitenant` and then converting like follows:

```sh
curl -X POST \
    http://localhost:4000/validate \
    -H 'Content-Type: application/json' \
    -d @./static/deployments/123456.json \
    > ./static/deployments/123456.bin
```

We can then test this new deployment:

```sh
curl http://localhost:4000/deployment/00000000-0000-0000-0000-000000000000/schema
```
