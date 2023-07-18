#!/usr/bin/env bash

set -e
set -u
set -o pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

# print its arguments in green
function info {
  echo  "> $(tput setaf 2)$*$(tput sgr0)"
}

function stop {
  info 'Stopping the agent'
  docker compose down agent
}

info 'Building a Docker image'
SELF_IMAGE_PATH="$(cd ../.. && nix build --no-link --print-out-paths '.#docker')"
SELF_IMAGE_NAME="$(cd ../.. && nix eval --raw '.#docker.imageName')"
SELF_IMAGE_TAG="$(cd ../.. && nix eval --raw '.#docker.imageTag')"
SELF_IMAGE="${SELF_IMAGE_NAME}:${SELF_IMAGE_TAG}"
export SELF_IMAGE

info 'Loading the Docker image'
docker load --quiet < "$SELF_IMAGE_PATH"

trap stop EXIT INT QUIT TERM

info 'Starting the services'
docker compose up --detach --wait agent grafana

info 'Running the benchmarks'
docker compose run --rm benchmark
