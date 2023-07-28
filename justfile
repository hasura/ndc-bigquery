POSTGRESQL_CONNECTION_STRING := "postgresql://postgres:password@localhost:64002"

CHINOOK_DEPLOYMENT := "static/chinook-deployment.json"

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
    -x 'run --bin ndc-postgres -- serve --configuration {{CHINOOK_DEPLOYMENT}}'

# watch the code and run the postgres-multitenant-gdc on changes
run-quickly: start-docker
  RUST_LOG=INFO \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo run --release --bin ndc-postgres -- serve --configuration {{CHINOOK_DEPLOYMENT}}'

# watch the code and run the postgres-multitenant-gdc on changes
watch-run: start-docker
  RUST_LOG=DEBUG \
    OTEL_SERVICE_NAME=postgres-agent \
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317 \
    OTEL_TRACES_SAMPLER=always_on \
    cargo watch -i "tests/snapshots/*" \
    -c \
    -x 'run --bin ndc-postgres -- serve --configuration {{CHINOOK_DEPLOYMENT}}'

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
    serve --configuration {{CHINOOK_DEPLOYMENT}}

# run postgres + jaeger
start-docker:
  # start jaeger, configured to listen to V3
  docker compose -f ../v3-engine/docker-compose.yaml up -d jaeger
  # start our local postgres
  docker compose up --wait

# run the v3 engine binary, pointing it at our multitenant agent
run-engine: start-docker
  @echo "http://localhost:3000/ for graphiql console"
  @echo "http://localhost:4002/ for jaeger console"
  # Run graphql-engine using static Chinook metadata
  # we expect the `v3-engine` repo to live next door to this one
  RUST_LOG=DEBUG cargo run --release \
    --manifest-path ../v3-engine/Cargo.toml \
    --bin engine -- \
    --metadata-path ./static/chinook-metadata.json

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

# run metadata service from ../v3-experiments repo
run-metadata-service:
  RUST_LOG=DEBUG cargo run --release \
    --manifest-path ../v3-experiments/Cargo.toml \
    --bin metadata-service

# regenerate our test metadata binary files using metadata service
regenerate-metadata-binaries:
  @echo "Please run 'just run-metadata-service' in another window before using this command"
  curl -X POST \
    -H 'Host: example.hasura.app' \
    -H 'Content-Type: application/json' \
    http://localhost:4000/validate \
    -d @./static/metadata/example.hasura.app/metadata.json \
      > ./static/metadata/example.hasura.app/metadata.bin

# regenerate deployment binary files from json
regenerate-deployment-binaries:
  @echo "Please run 'just run-multitenant' in another window before using this command"

  @echo "Regenerating benchmark deployment binary"
  curl -X POST \
    http://localhost:4000/validate \
    -H 'Content-Type: application/json' \
    -d @./benchmarks/component/deployments/9f532406-b0c8-4c17-a58a-64286c1e1fd6.json \
    > ./benchmarks/component/deployments/9f532406-b0c8-4c17-a58a-64286c1e1fd6.bin

  @echo "Regenerating test deployment binary"
  curl -X POST \
    http://localhost:4000/validate \
    -H 'Content-Type: application/json' \
    -d @./static/deployments/88011674-8513-4d6b-897a-4ab856e0bb8a.json \
    > ./static/deployments/88011674-8513-4d6b-897a-4ab856e0bb8a.bin
