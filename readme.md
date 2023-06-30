# Postgres Native Data Connector

## Build

### Prequisites

1. Install [rustup](https://www.rust-lang.org/tools/install).
2. Install additional tools: `cargo install watch rustfmt`, `rustup component add rust-analyzer`.
3. Install [docker](https://www.docker.com/)

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
