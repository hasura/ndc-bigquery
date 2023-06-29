POSTGRESQL_CONNECTION_STRING=postgresql://postgres:password@localhost:64002

# we use this port because it's hardcoded in the metadata too
POSTGRES_DC_PORT=8666

.PHONY: dev
dev:
	cargo watch -c -x test -x run

.PHONY: start-docker
## start-docker: run postgres + jaeger
start-docker:
	# start jaeger, configured to listen to V3
	docker-compose -f ../v3-experiments/crates/engine/services/dev.docker-compose.yaml up -d jaeger
	# start our local postgres
	docker-compose up --wait

.PHONY: run-v3
run-v3: start-docker
	@echo "http://localhost:3000/ for graphiql console"
	@echo "http://localhost:4002/ for jaeger console"
	# Run graphql-engine using static Chinook metadata
	# we expect the `v3-experiments` repo to live next door to this one
	RUST_LOG=DEBUG cargo run --release \
		--manifest-path ../v3-experiments/Cargo.toml --bin engine -- \
    --metadata-path ./static/metadata-example.json \
    --data-connectors-config ./static/data-connectors-config-example.json \
    --secrets-path ./static/secrets-example.json

.PHONY: run-postgres-dc
## run-postgres-dc: run-postgres-dc, pointing it at local postgres etc:w
run-postgres-dc: start-docker
	RUST_LOG=DEBUG \
		PORT=${POSTGRES_DC_PORT} \
		POSTGRESQL_CONNECTION_STRING=${POSTGRESQL_CONNECTION_STRING} \
		cargo run --release

.PHONY: repl-postgres
## repl-postgres: start a postgres docker image and connect to it using psql
repl-postgres:
	@docker compose up --wait postgres;
	psql ${POSTGRES_CONNECTION_STRING}
