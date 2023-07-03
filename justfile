POSTGRESQL_CONNECTION_STRING := "postgresql://postgres:password@localhost:64002"

# we use this port because it's hardcoded in the metadata too
POSTGRES_DC_PORT := "8666"

# this is hardcoded in the V3 metadata
POSTGRES_MULTITENANT_DC_PORT := "8081"

# watch the code and re-run on changes
dev: start-docker
  RUST_LOG=DEBUG \
    PORT={{POSTGRES_DC_PORT}} \
    POSTGRESQL_CONNECTION_STRING={{POSTGRESQL_CONNECTION_STRING}} \
    cargo watch -i "tests/snapshots/*" -c -C ./crates/postgres-ndc -x test -x run

# watch the multitenant code and re-run on changes
dev-multitenant:
  RUST_LOG=DEBUG \
    cargo watch -c -C ./crates/postgres-multitenant-ndc -x test

# run postgres + jaeger
start-docker:
  # start jaeger, configured to listen to V3
  docker compose -f ../v3-experiments/crates/engine/services/dev.docker-compose.yaml up -d jaeger
  # start our local postgres
  docker compose up --wait

run-v3: start-docker
  @echo "http://localhost:3000/ for graphiql console"
  @echo "http://localhost:4002/ for jaeger console"
  # Run graphql-engine using static Chinook metadata
  # we expect the `v3-experiments` repo to live next door to this one
  RUST_LOG=DEBUG cargo run --release \
    --manifest-path ../v3-experiments/Cargo.toml \
    --bin engine -- \
    --metadata-dir ./static/ \
    --secrets-path ./static/secrets-example.json

run-v3-multitenant: start-docker
  @echo "http://localhost:4002/ for jaeger console"
  # Run graphql-engine using static Chinook metadata
  # we expect the `v3-experiments` repo to live next door to this one
  # we should also set up --otlp-endpoint to point at Jaeger
  RUST_LOG=DEBUG cargo run --release \
    --manifest-path ../v3-experiments/Cargo.toml \
    --bin multitenant -- \
    --metadata-dir ../v3-experiments/metadata/ \

# run-postgres-dc, pointing it at local postgres etc
run-postgres-dc: start-docker
  RUST_LOG=DEBUG \
    PORT={{POSTGRES_DC_PORT}} \
    POSTGRESQL_CONNECTION_STRING={{POSTGRESQL_CONNECTION_STRING}} \
    cargo run --release --bin postgres-ndc

# run-postgres-multitenant-dc, pointing it at local postgres etc
run-postgres-multitenant-dc: start-docker
  RUST_LOG=DEBUG \
    PORT={{POSTGRES_MULTITENANT_DC_PORT}} \
    cargo run --release \
    --bin postgres-multitenant-ndc -- \
    --deployments-dir ./static/deployments/

# start a postgres docker image and connect to it using psql
repl-postgres:
  @docker compose up --wait postgres
  psql {{POSTGRESQL_CONNECTION_STRING}}

# run a request to check multitenant is working
test-multitenant:
  curl -X POST \
    -H 'Host: example.hasura.app' \
      -H 'Content-Type: application/json' \
    http://localhost:3000/graphql \
    -d '{ "query": "query { AlbumByID(AlbumId: 1) { Title } } " }'
