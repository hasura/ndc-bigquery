POSTGRESQL_CONNECTION_STRING := "postgresql://postgres:password@localhost:64002"

CHINOOK_DEPLOYMENT := "static/chinook-deployment.json"

# this is hardcoded in chinook-metadata.json
POSTGRES_DC_PORT := "8100"

# run the connector
run: start-docker
  RUST_LOG=INFO \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo run --release -- serve --configuration {{CHINOOK_DEPLOYMENT}}

# watch the code, then test and re-run on changes
dev: start-docker
  RUST_LOG=INFO \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo watch -i "tests/snapshots/*" \
    -c \
    -x test \
    -x clippy \
    -x 'run -- serve --configuration {{CHINOOK_DEPLOYMENT}}'

# watch the code, and re-run on changes
watch-run: start-docker
  RUST_LOG=DEBUG \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo watch -i "tests/snapshots/*" \
    -c \
    -x 'run -- serve --configuration {{CHINOOK_DEPLOYMENT}}'

# Run the server and produce a flamegraph profile
flamegraph: start-docker
  RUST_LOG=DEBUG \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo flamegraph --dev -- \
    serve --configuration {{CHINOOK_DEPLOYMENT}}

# run postgres + jaeger
start-docker:
  # start jaeger, configured to listen to V3
  docker compose -f ../v3-engine/docker-compose.yaml up -d jaeger
  # start our local postgres
  docker compose up --wait

# run the v3 engine binary, pointing it at our connector
run-engine: start-docker
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

# run all tests
test: start-docker
  RUST_LOG=DEBUG \
    cargo test

# run a standard request to check the service correctly integrates with the engine
test-integrated:
  curl -X POST \
    -H 'Host: example.hasura.app' \
    -H 'Content-Type: application/json' \
    http://localhost:3000/graphql \
    -d '{ "query": "query { AlbumByID(id: 1) { title } } " }'

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
