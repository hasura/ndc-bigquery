#!/usr/bin/env bash
set -e -u

EXECUTABLE="$1"
CONNECTION_STRING="$2"
CHINOOK_DEPLOYMENT="$3"

# start config server
cargo run --bin "${EXECUTABLE}" --quiet -- configuration serve &
CONFIGURATION_SERVER_PID=$!
trap "kill $CONFIGURATION_SERVER_PID" EXIT
./scripts/wait-until --timeout=30 --report -- nc -z localhost 9100
if ! kill -0 "$CONFIGURATION_SERVER_PID"; then
  echo >&2 'The server stopped abruptly.'
  exit 1
fi

# name for temp file
CHINOOK_DEPLOYMENT_OLD="${CHINOOK_DEPLOYMENT}.old"

# make a copy of the old file
cp "${CHINOOK_DEPLOYMENT}" "${CHINOOK_DEPLOYMENT_OLD}"

# pass connection string to config server to generate initial deployment from introspection
curl -fsS http://localhost:9100 \
  | jq --arg postgres_database_url "${CONNECTION_STRING}" '. + {"postgres_database_url": $postgres_database_url, "version": 1, "metadata": {}, "aggregate_functions": {}}' \
  | curl -fsS http://localhost:9100 -H 'Content-Type: application/json' -d @- \
  | jq . \
  > "${CHINOOK_DEPLOYMENT}"

# grab .metadata.native_queries from the old file, and put it into original file
cat "${CHINOOK_DEPLOYMENT}" \
  | jq --arg old_native_queries "$(cat "${CHINOOK_DEPLOYMENT_OLD}" | jq '.metadata.native_queries')" '.metadata.native_queries |= ($old_native_queries | fromjson)' \
  > "${CHINOOK_DEPLOYMENT}"

# grab .postgres_database_url from the old file, and put it into original file
cat "${CHINOOK_DEPLOYMENT}" \
  | jq --arg old_url "$(cat "${CHINOOK_DEPLOYMENT_OLD}" | jq '.postgres_database_url')" '.postgres_database_url |= ($old_url | fromjson)' \
  > "${CHINOOK_DEPLOYMENT}"

# remote temp file
rm "${CHINOOK_DEPLOYMENT_OLD}"
