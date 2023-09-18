# PostgreSQL Native Data Connector

## Development

### Prequisites

1. Install [rustup](https://www.rust-lang.org/tools/install)
2. Install additional tools:
   - `cargo install cargo-watch cargo-insta`
3. Install [just](https://github.com/casey/just)
4. Install [Prettier](https://prettier.io/)
5. Install [Docker](https://www.docker.com/)
6. Install protoc. Here are a few options:
   - `brew install protobuf`
   - `apt-get install protobuf-compiler`
   - `dnf install protobuf-compiler`
7. Clone [v3-engine](https://github.com/hasura/v3-engine) in a directory near this one:
   ```
   (cd .. && git clone git@github.com:hasura/v3-engine.git)
   ```

### Compile

```
cargo build
```

### Run

Run the PostgreSQL connector with:

```
just run
```

### Code

1. Start the sample chinook postgres db, compile, run tests, and rerun server on file changes: `just dev`
2. Query the connector via curl:
   ```
   curl -H "Content-Type: application/json" \
     --data "@crates/tests/tests-common/goldenfiles/select_where_variable.json" \
     http://localhost:8100/query \
     | jq
   ```

Among the docker containers is a Jaeger instance for tracing/debugging, accessible at http://127.0.0.1:4002.

See [architecture.md](./architecture.md) for an idea of the general structure.

### Debug

See [debugging.md](./debugging.md).

### Other PostgreSQL flavours

You can also run `just dev-citus`, `just dev-cockroach` or `just dev-aurora`.

Aurora runs against a static AWS instance, so you'll need to set the `AURORA_CONNECTION_STRING` environment variable
to a valid connection string.

### Profile

We can produce a flamegraph using `just flamegraph` using [flamegraph-rs](https://github.com/flamegraph-rs/flamegraph). Follow the installation instructions.

### Benchmark

See [./benchmarks/component/README.md](./benchmarks/component/README.md).

A benchmark history can be viewed [here](https://hasura.github.io/postgres-ndc/dev/bench).

## Production

We ship the various connectors as Docker images, built with Nix.

### Build

You can build each one with `nix build '.#ndc-<flavor>-docker'`, which will build a Docker tarball.

For example, to build the PostgreSQL image and load it into your Docker image registry, run:

```
gunzip < "$(nix build --no-warn-dirty --no-link --print-out-paths '.#ndc-postgres-docker')" | docker load
```

This will build an image named `ghcr.io/hasura/ndc-postgres:dev`.

As a shortcut, `just build-docker-with-nix` will build the PostgreSQL image.

### Run

Set up the PostgreSQL database you wish to connect to. For example, in order to create a transient database loaded with
Chinook, you could use the following Docker Compose configuration:

```yaml
services:
  db:
    image: postgres
    platform: linux/amd64
    environment:
      POSTGRES_PASSWORD: "password"
    volumes:
      - type: tmpfs
        target: /var/lib/postgresql/data
      - type: bind
        source: ./static/chinook-postgres.sql
        target: /docker-entrypoint-initdb.d/chinook-postgres.sql
        read_only: true
    healthcheck:
      test:
        - CMD-SHELL
        - psql -U "$${POSTGRES_USER:-postgres}" < /dev/null && sleep 5 && psql -U "$${POSTGRES_USER:-postgres}" < /dev/null
      start_period: 5s
      interval: 5s
      timeout: 10s
      retries: 20
```

Next, create a configuration file. For the example above, you can do this by copying `./static/chinook-deployment.json`
to a new file (e.g. `./deployment.json`) and changing the `"connection_uris"` to
`["postgresql://postgres:password@db"]`.

Once that's set up, you can set up the connector to point at your PostgreSQL database:

```yaml
services:
  connector:
    image: ghcr.io/hasura/ndc-postgres:dev
    command:
      - serve
      - --configuration=/deployment.json
    ports:
      - 8100:8100
    volumes:
      - type: bind
        source: ./deployment.json
        target: /deployment.json
        read_only: true
    healthcheck:
      test:
        - CMD
        - ndc-postgres
        - check-health
      start_period: 5s
      interval: 5s
      timeout: 10s
      retries: 3
    depends_on:
      db:
        condition: service_healthy
```

Running `docker compose up --detach --wait` will start the container running on port 8100.

Note that the `healthcheck` section refers to the binary `ndc-postgres`. This will vary per connector flavor.

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

1. Create a new file under `crates/tests/tests-common/goldenfiles/<your-test-name>.json`
2. Create a new test in `crates/connectors/ndc-postgres/tests/query_tests.rs` that looks like this:
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

1. Create a new folder under `crates/query-engine/translation/tests/goldenfiles/<your-test-name>/`
2. Create `request.json` and `tables.json` files in that folder to specify your
   request
3. Create a new test in `crates/query-engine/translation/tests/tests.rs` that looks like this:
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
