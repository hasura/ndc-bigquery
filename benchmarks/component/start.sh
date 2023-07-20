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

info 'Starting the services'
docker compose up --detach --wait agent grafana
