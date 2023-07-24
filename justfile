POSTGRESQL_CONNECTION_STRING := "postgresql://postgres:password@localhost:64002"

# this is hardcoded in the V3 metadata
POSTGRES_DC_PORT := "8100"

# watch the code and re-run on changes
dev: start-docker
  RUST_LOG=INFO \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo watch -i "tests/snapshots/*" \
    -c \
    -x test \
    -x clippy \
    -x 'run --bin ndc-postgres -- --configuration static/deployments/88011674-8513-4d6b-897a-4ab856e0bb8a.json'

# watch the code and run the postgres-multitenant-gdc on changes
run-quickly: start-docker
  RUST_LOG=INFO \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo run --release --bin ndc-postgres -- --configuration static/deployments/88011674-8513-4d6b-897a-4ab856e0bb8a.json

# watch the code and run the postgres-multitenant-gdc on changes
watch-run: start-docker
  RUST_LOG=DEBUG \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo watch -i "tests/snapshots/*" \
    -c \
    -x 'run --bin ndc-postgres -- --configuration static/deployments/88011674-8513-4d6b-897a-4ab856e0bb8a.json'

# watch the code and re-run on changes
run-multitenant: start-docker
  RUST_LOG=DEBUG \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo run --bin ndc-postgres-multitenant -- --deployments-dir ./static/deployments

# Run the server and produce a flamegraph profile
flamegraph: start-docker
  RUST_LOG=DEBUG \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo flamegraph --dev --bin ndc-postgres -- \
    --configuration static/deployments/88011674-8513-4d6b-897a-4ab856e0bb8a.json

# run postgres + jaeger
start-docker:
  # start jaeger, configured to listen to V3
  docker compose -f ../v3-experiments/crates/engine/services/dev.docker-compose.yaml up -d jaeger
  # start our local postgres
  docker compose up --wait

# run the regular V3 binary, pointing it at our multitenant agent
run-v3: start-docker
  @echo "http://localhost:3000/ for graphiql console"
  @echo "http://localhost:4002/ for jaeger console"
  # Run graphql-engine using static Chinook metadata
  # we expect the `v3-experiments` repo to live next door to this one
  RUST_LOG=DEBUG cargo run --release \
    --manifest-path ../v3-experiments/Cargo.toml \
    --bin engine -- \
    --data-connectors-config ./static/data-connectors-config-example.json \
    --metadata-path ./static/metadata-example.json \
    --secrets-path ./static/secrets-example.json

# run the V3 multitenant binary, pointing it at our multitenant agent
run-v3-multitenant: start-docker
  @echo "http://localhost:4002/ for jaeger console"
  # Run graphql-engine using static Chinook metadata
  # we expect the `v3-experiments` repo to live next door to this one
  # we should also set up --otlp-endpoint to point at Jaeger
  RUST_LOG=DEBUG cargo run --release \
    --manifest-path ../v3-experiments/Cargo.toml \
    --bin multitenant -- \
    --metadata-dir ./static/metadata/ \

# run-postgres-ndc, pointing it at local postgres etc
run-postgres-ndc: start-docker
  RUST_LOG=DEBUG \
    PORT={{POSTGRES_DC_PORT}} \
    cargo run --release \
    --bin postgres-multitenant-ndc -- \
    --deployments-dir ./static/deployments/

# start a postgres docker image and connect to it using psql
repl-postgres:
  @docker compose up --wait postgres
  psql {{POSTGRESQL_CONNECTION_STRING}}

# run a standard request to check multitenant is working
test-multitenant:
  curl -X POST \
    -H 'Host: example.hasura.app' \
    -H 'Content-Type: application/json' \
    -H "X-B3-TraceId: 5f868e8d0968ddff87a747e592d13cec-742de8013df0852f-0" \
    http://localhost:3000/graphql \
    -d '{ "query": "query { AlbumByID(AlbumId: 1) { Title } } " }'

# run all tests
test: start-docker
  RUST_LOG=DEBUG \
    cargo test

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

# check the nix build works
build-with-nix:
  nix build --print-build-logs

# check the docker build works
build-docker-with-nix:
  nix build .#docker --print-build-logs
  docker load < ./result
