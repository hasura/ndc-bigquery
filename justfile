set shell := ["bash", "-c"]

CONNECTOR_IMAGE_NAME := "ghcr.io/hasura/postgres-agent-rs"
CONNECTOR_IMAGE_TAG := "dev"
CONNECTOR_IMAGE := CONNECTOR_IMAGE_NAME + ":" + CONNECTOR_IMAGE_TAG
POSTGRESQL_CONNECTION_STRING := "postgresql://postgres:password@localhost:64002"
CHINOOK_DEPLOYMENT := "static/chinook-deployment.json"

# Notes:
# * Building Docker images will not work on macOS.
#   You can use `main` instead, by running:
#     just --set CONNECTOR_IMAGE_TAG dev-main <targets>

# run the connector
run: start-dependencies
  RUST_LOG=INFO \
    cargo run --release -- serve --configuration {{CHINOOK_DEPLOYMENT}}

# run the connector inside a Docker image
run-in-docker: build-docker-with-nix start-dependencies
  #!/usr/bin/env bash
  set -e -u -o pipefail

  configuration_file="$(mktemp)"
  trap 'rm -f "$configuration_file"' EXIT

  echo '> Generating the configuration...'
  docker run \
    --name=postgres-ndc-configuration \
    --rm \
    --detach \
    --platform=linux/amd64 \
    --net='postgres-ndc_default' \
    --publish='9100:9100' \
    {{CONNECTOR_IMAGE}} \
    configuration serve
  trap 'docker stop postgres-ndc-configuration' EXIT
  CONFIGURATION_SERVER_URL='http://localhost:9100/'
  ./scripts/wait-until --timeout=30 --report -- nc -z localhost 9100
  curl -fsS "$CONFIGURATION_SERVER_URL" \
    | jq --arg postgres_database_url 'postgresql://postgres:password@postgres' '. + {"postgres_database_url": $postgres_database_url}' \
    | curl -fsS "$CONFIGURATION_SERVER_URL" -H 'Content-Type: application/json' -d @- \
    > "$configuration_file"

  echo '> Starting the server...'
  docker run \
    --name=postgres-ndc \
    --rm \
    --interactive \
    --tty \
    --platform=linux/amd64 \
    --net='postgres-ndc_default' \
    --publish='8100:8100' \
    --env=RUST_LOG='INFO' \
    --mount="type=bind,source=${configuration_file},target=/deployment.json,readonly=true" \
    {{CONNECTOR_IMAGE}} \
    serve \
    --configuration='/deployment.json'

# watch the code, then test and re-run on changes
dev: start-dependencies
  RUST_LOG=INFO \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_SERVICE_NAME=postgres-ndc \
    cargo watch -i "**/snapshots/*" \
    -c \
    -x 'test --exclude e2e-tests --workspace' \
    -x clippy \
    -x 'run -- serve --configuration {{CHINOOK_DEPLOYMENT}}'

# watch the code, and re-run on changes
watch-run: start-dependencies
  RUST_LOG=DEBUG \
    cargo watch -i "tests/snapshots/*" \
    -c \
    -x 'run -- serve --configuration {{CHINOOK_DEPLOYMENT}}'

# Run ndc-postgres with rust-gdb.
debug: start-dependencies
  cargo build
  RUST_LOG=DEBUG \
    rust-gdb --args target/debug/ndc-postgres serve --configuration {{CHINOOK_DEPLOYMENT}}

# Run the server and produce a flamegraph profile
flamegraph: start-dependencies
  RUST_LOG=DEBUG \
    cargo flamegraph --dev -- \
    serve --configuration {{CHINOOK_DEPLOYMENT}}

# build everything
build:
  cargo build --all-targets

# run all tests
test: start-dependencies
  RUST_LOG=DEBUG \
    cargo test --workspace --exclude=e2e-tests

# re-generate the deployment configuration file
generate-chinook-configuration: build
  #!/usr/bin/env bash
  set -e -u

  cargo run --quiet -- configuration serve &
  CONFIGURATION_SERVER_PID=$!
  trap "kill $CONFIGURATION_SERVER_PID" EXIT
  ./scripts/wait-until --timeout=30 --report -- nc -z localhost 9100
  if ! kill -0 "$CONFIGURATION_SERVER_PID"; then
    echo >&2 'The server stopped abruptly.'
    exit 1
  fi
  curl -fsS http://localhost:9100 \
    | jq --arg postgres_database_url '{{POSTGRESQL_CONNECTION_STRING}}' '. + {"postgres_database_url": $postgres_database_url}' \
    | curl -fsS http://localhost:9100 -H 'Content-Type: application/json' -d @- \
    | jq . \
    > '{{CHINOOK_DEPLOYMENT}}'

# run postgres + jaeger
start-dependencies:
  # start jaeger, configured to listen to V3
  docker compose -f ../v3-engine/docker-compose.yaml up -d jaeger
  # start postgres
  docker compose up --wait postgres

# run prometheus + grafana
start-metrics:
  @echo "http://localhost:3001/ for grafana console"
  docker compose up --wait prometheus grafana

# run the v3 engine binary, pointing it at our connector
run-engine: start-dependencies
  @echo "http://localhost:3000/ for graphiql console"
  @echo "http://localhost:4002/ for jaeger console"
  # Run graphql-engine using static Chinook metadata
  # we expect the `v3-engine` repo to live next door to this one
  RUST_LOG=DEBUG cargo run --release \
    --manifest-path ../v3-engine/Cargo.toml \
    --bin engine -- \
    --metadata-path ./static/chinook-metadata.json

# start a postgres docker image and connect to it using psql
repl-postgres:
  @docker compose up --wait postgres
  psql {{POSTGRESQL_CONNECTION_STRING}}

# run `clippy` linter
lint *FLAGS:
  cargo clippy {{FLAGS}}

lint-apply *FLAGS:
  cargo clippy --fix {{FLAGS}}

# run rustfmt on everything
format:
  cargo fmt --all

# is everything formatted?
format-check:
  cargo fmt --all -- --check

# Format all json test files
format-tests:
  ./scripts/format-with-jq.sh crates/ndc-postgres/tests/goldenfiles/*.json
  ./scripts/format-with-jq.sh crates/query-engine/tests/goldenfiles/*/*.json

# check the nix build works
build-with-nix:
  nix build --print-build-logs

# check the docker build works
build-docker-with-nix:
  #!/usr/bin/env bash
  if [[ '{{CONNECTOR_IMAGE_TAG}}' == 'dev' ]]; then
    echo 'nix build | docker load'
    docker load < "$(nix build --no-link --print-out-paths '.#dockerDev')"
  fi
