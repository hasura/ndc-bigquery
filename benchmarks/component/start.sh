#!/usr/bin/env bash

set -e
set -u
set -o pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

# prints its arguments to STDERR in green
function info {
  echo >&2 "> $(tput setaf 2)$*$(tput sgr0)"
}

if [[ -e ./agent.pid ]]; then
  info 'Stopping the agent from a previous run'
  AGENT_PID="$(< ./agent.pid)"
  kill "$AGENT_PID" && wait "$AGENT_PID" || :
  rm ./agent.pid
fi

info 'Building the agent'
cargo build --release

info 'Starting the dependencies'
docker compose up --detach --wait postgres grafana
POSTGRESQL_SOCKET="$(docker compose port postgres 5432)"

info 'Generating the deployment configuration'
mkdir -p generated
cargo run --quiet --release -- configuration serve &
AGENT_PID=$!
echo "$AGENT_PID" > ./agent.pid
sleep 1
if ! kill -0 "$AGENT_PID"; then
  echo >&2 'The agent stopped abruptly. Take a look at agent.log for details.'
  exit 1
fi
curl -fsS http://localhost:9100 \
  | jq --arg postgres_database_url "postgresql://postgres:password@${POSTGRESQL_SOCKET}" '. + {"postgres_database_url": $postgres_database_url}' \
  | curl -fsS http://localhost:9100 -H 'Content-Type: application/json' -d @- \
  > ./generated/deployment.json
kill "$AGENT_PID"
rm -f ./agent.pid

info 'Starting the agent'
cargo run --quiet --release -- serve --configuration=./generated/deployment.json >& agent.log &
AGENT_PID=$!
echo "$AGENT_PID" > ./agent.pid
echo >&2 "The agent is running with PID ${AGENT_PID}"
sleep 1
if ! kill -0 "$AGENT_PID"; then
  echo >&2 'The agent stopped abruptly. Take a look at agent.log for details.'
  exit 1
fi
