#!/usr/bin/env bash

set -e
set -u
set -o pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

# prints its arguments to STDERR in green
function info {
  echo >&2 "> $(tput setaf 2)$*$(tput sgr0)"
}

info 'Building a Docker image'
SELF_IMAGE_PATH="$(cd ../.. && nix --no-warn-dirty build --no-link --print-out-paths '.#dockerDev')"

info 'Loading the Docker image'
docker load --quiet < "$SELF_IMAGE_PATH"

info 'Starting the dependencies'
docker compose up --detach --wait postgres grafana

info 'Generating the deployment configuration'
mkdir -p generated
docker compose run generate-configuration > generated/deployment.json

info 'Starting the agent'
docker compose up --detach --wait agent
