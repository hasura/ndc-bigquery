# Postgres Native Data Connector

## Build

### Prequisites

1. Install [rustup](https://www.rust-lang.org/tools/install).
2. Install additional tools: `cargo install watch rustfmt`, `rustup component add rust-analyzer`.
3. Install [docker](https://www.docker.com/)
4. Install protoc. Here are a few options:
   - `brew install protobuf`
   - `apt-get install protobuf-compiler`
   - `dnf install protobuf-compiler`
5. Clone v3 experiments in a directory near this one: `(cd .. && git clone git@github.com:hasura/v3-experiments.git)`

### Compile

```
cargo build
```

### Run

```
make run-postgres-dc
```

### Develop

1. Start the database sample: `make start-docker`
2. Compile, run tests, and rerun server on file changes: `make dev`
3. Query the connector via curl: `curl -X POST http://localhost:8666/query`

## General structure

- Entry point: `src/main.rs`
- Connector state: `src/connector.rs`
- Routing: `src/routes/`
- Input and output types: `src/types/`
- Compiler phases: `src/phases/`:
   - Translation from query request to sql ast: `src/phases/translation.rs`
   - Translation from sql_ast to sql_string: `src/phases/translation/`
   - Execution of the plan against postgres: `src/phases/execution.rs`
- Unit and integration tests: `tests/`

## Example

1. Run `make start-docker`
2. Run `make dev`
3. Run `make run-v3`
4. Connect to GraphiQL at http://localhost:3000 and run a query:

   ```graphql
   query {
     AlbumByID(AlbumId: 35) {
       Title
     }
   }
   ```
