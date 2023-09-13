set shell := ["bash", "-c"]

CONNECTOR_IMAGE_NAME := "ghcr.io/hasura/ndc-postgres"
CONNECTOR_IMAGE_TAG := "dev"
CONNECTOR_IMAGE := CONNECTOR_IMAGE_NAME + ":" + CONNECTOR_IMAGE_TAG

POSTGRESQL_CONNECTION_STRING := "postgresql://postgres:password@localhost:64002"
POSTGRES_CHINOOK_DEPLOYMENT := "static/chinook-deployment.json"

COCKROACH_CONNECTION_STRING := "postgresql://postgres:password@localhost:64003"
COCKROACH_CHINOOK_DEPLOYMENT := "static/cockroach/chinook-deployment.json"

CITUS_CONNECTION_STRING := "postgresql://postgres:password@localhost:64004?sslmode=disable"
CITUS_CHINOOK_DEPLOYMENT := "static/citus/chinook-deployment.json"

AURORA_CONNECTION_STRING := env_var_or_default('AURORA_CONNECTION_STRING', '')
AURORA_CHINOOK_DEPLOYMENT := "static/aurora/chinook-deployment.json"
AURORA_CHINOOK_DEPLOYMENT_TEMPLATE := "static/aurora/chinook-deployment-template.json"

# Notes:
# * Building Docker images will not work on macOS.
#   You can use `main` instead, by running:
#     just --set CONNECTOR_IMAGE_TAG dev-main <targets>

# check everything
check: build lint format-check test

# run the connector
run: start-dependencies
  RUST_LOG=INFO \
    cargo run --bin ndc-postgres --release -- serve --configuration {{POSTGRES_CHINOOK_DEPLOYMENT}}

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
    -x 'test -p query-engine -p ndc-postgres' \
    -x clippy \
    -x 'run --bin ndc-postgres -- serve --configuration {{POSTGRES_CHINOOK_DEPLOYMENT}}'

# watch the code, then test and re-run on changes
dev-cockroach: start-cockroach-dependencies
  RUST_LOG=INFO \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_SERVICE_NAME=cockroach-ndc \
    cargo watch -i "**/snapshots/*" \
    -c \
    -x 'test -p query-engine -p ndc-cockroach' \
    -x clippy \
    -x 'run --bin ndc-cockroach -- serve --configuration {{COCKROACH_CHINOOK_DEPLOYMENT}}'

# watch the code, then test and re-run on changes
dev-citus: start-citus-dependencies
  RUST_LOG=INFO \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_SERVICE_NAME=citus-ndc \
    cargo watch -i "**/snapshots/*" \
    -c \
    -x 'test -p query-engine -p ndc-citus' \
    -x clippy \
    -x 'run --bin ndc-citus -- serve --configuration {{CITUS_CHINOOK_DEPLOYMENT}}'

# Run postgres, testing against external DBs like Aurora
test-other-dbs: create-aurora-deployment start-dependencies
  RUST_LOG=INFO \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_SERVICE_NAME=postgres-ndc \
    cargo watch -i "**/snapshots/*" \
    -c \
    -x 'test -p other-db-tests' \
    -x clippy \
    -x 'run --bin ndc-postgres -- serve --configuration {{AURORA_CHINOOK_DEPLOYMENT}}'

# watch the code, and re-run on changes
watch-run: start-dependencies
  RUST_LOG=DEBUG \
    cargo watch -i "tests/snapshots/*" \
    -c \
    -x 'run -- serve --configuration {{POSTGRES_CHINOOK_DEPLOYMENT}}'

# Run ndc-postgres with rust-gdb.
debug: start-dependencies
  cargo build
  RUST_LOG=DEBUG \
    rust-gdb --args target/debug/ndc-postgres serve --configuration {{POSTGRES_CHINOOK_DEPLOYMENT}}

# Run the server and produce a flamegraph profile
flamegraph: start-dependencies
  RUST_LOG=DEBUG \
    cargo flamegraph --dev -- \
    serve --configuration {{POSTGRES_CHINOOK_DEPLOYMENT}}

# build everything
build:
  cargo build --all-targets

# run all tests
test: start-dependencies start-cockroach-dependencies start-citus-dependencies create-aurora-deployment
  RUST_LOG=DEBUG \
    cargo test -p query-engine -p ndc-postgres -p ndc-cockroach -p ndc-citus -p other-db-tests


# re-generate the deployment configuration file
generate-chinook-configuration: build start-dependencies start-cockroach-dependencies start-citus-dependencies
  ./scripts/generate-chinook-configuration.sh "ndc-postgres" '{{POSTGRESQL_CONNECTION_STRING}}' '{{POSTGRES_CHINOOK_DEPLOYMENT}}'
  if [[ -n '{{AURORA_CONNECTION_STRING}}' ]]; then \
    ./scripts/generate-chinook-configuration.sh "ndc-postgres" '{{AURORA_CONNECTION_STRING}}' '{{AURORA_CHINOOK_DEPLOYMENT_TEMPLATE}}'; \
  fi

  # regenerate aurora deployment from template we've just updated
  just create-aurora-deployment

# run postgres + jaeger
start-dependencies:
  # start jaeger, configured to listen to V3
  docker compose -f ../v3-engine/docker-compose.yaml up -d jaeger
  # start postgres
  docker compose up --wait postgres

# run cockroach + jaeger
start-cockroach-dependencies:
  # start jaeger, configured to listen to V3
  docker compose -f ../v3-engine/docker-compose.yaml up -d jaeger
  # start cockroach
  docker compose up --wait cockroach

# run citus + jaeger
start-citus-dependencies:
  # start jaeger, configured to listen to V3
  docker compose -f ../v3-engine/docker-compose.yaml up -d jaeger
  # start citus
  docker compose up --wait citus

# setup aurora + jaeger
# aurora is a big different, the 'setup' step is taking the
# `AURORA_CONNECTION_STRING` and inserting it into a new copy of the deployment
create-aurora-deployment:
  # start jaeger, configured to listen to V3
  docker compose -f ../v3-engine/docker-compose.yaml up -d jaeger
  # splice `AURORA_CONNECTION_STRING` into
  cat {{ AURORA_CHINOOK_DEPLOYMENT_TEMPLATE }} \
    | jq '.postgres_database_url=(env | .AURORA_CONNECTION_STRING)' \
    | prettier --parser=json \
    > {{ AURORA_CHINOOK_DEPLOYMENT }}

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

# start a cockroach docker image and connect to it using psql
repl-cockroach:
  @docker compose up --wait cockroach
  psql {{COCKROACH_CONNECTION_STRING}}

# start a citus docker image and connect to it using psql
repl-citus:
  @docker compose up --wait citus
  psql {{CITUS_CONNECTION_STRING}}

# run `clippy` linter
lint *FLAGS:
  cargo clippy {{FLAGS}}

lint-apply *FLAGS:
  cargo clippy --fix {{FLAGS}}

# reformat everything
format:
  cargo fmt --all
  ! command -v nixpkgs-fmt > /dev/null || nixpkgs-fmt .
  prettier --write .

# is everything formatted?
format-check:
  cargo fmt --all --check
  ! command -v nixpkgs-fmt > /dev/null || nixpkgs-fmt --check .
  prettier --check .

# check the nix builds work
build-with-nix:
  nix build --no-warn-dirty --print-build-logs '.#ndc-postgres' '.#ndc-cockroach' '.#ndc-citus'

# check the docker build works
build-docker-with-nix:
  #!/usr/bin/env bash
  if [[ '{{CONNECTOR_IMAGE_TAG}}' == 'dev' ]]; then
    echo 'nix build .#ndc-postgres-docker | gunzip | docker load'
    gunzip < "$(nix build --no-warn-dirty --no-link --print-out-paths '.#ndc-postgres-docker')" | docker load
  fi

# check the Postgres arm64 docker build works
build-aarch64-docker-with-nix:
  #!/usr/bin/env bash
  if [[ '{{CONNECTOR_IMAGE_TAG}}' == 'dev' ]]; then
    echo 'nix build .#ndc-postgres-docker-aarch64-linux | gunzip | docker load'
    gunzip < "$(nix build --no-warn-dirty --no-link --print-out-paths --system aarch64-linux '.#ndc-postgres-docker-aarch64-linux')" | docker load
  fi

# check the Cockroach arm64 docker build works
build-cockroach-aarch64-docker-with-nix:
  #!/usr/bin/env bash
  if [[ '{{CONNECTOR_IMAGE_TAG}}' == 'dev' ]]; then
    echo 'nix build .#ndc-cockroach-docker-aarch64-linux | gunzip | docker load'
    gunzip < "$(nix build --no-warn-dirty --no-link --print-out-paths --system aarch64-linux '.#ndc-cockroach-docker-aarch64-linux')" | docker load
  fi
