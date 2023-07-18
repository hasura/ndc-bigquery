#!/usr/bin/env bash

set -e
set -u
set -o pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

# prints its arguments to STDERR in green
function info {
  echo >&2 "> $(tput setaf 2)$*$(tput sgr0)"
}

function stop {
  info 'Stopping the agent'
  docker compose down agent
}

if [[ $# -ne 1 ]]; then
  echo >&2 "Usage: ${BASH_SOURCE[0]} BENCHMARK"
  echo >&2
  echo >&2 "Benchmarks:"
  ls ./benchmarks | sed 's/^/  - /'
  exit 2
fi

BENCHMARK="$1"
if [[ ! -f "./benchmarks/$BENCHMARK" ]]; then
  echo >&2 "ERROR: Unknown benchmark: $BENCHMARK"
  echo >&2
  echo >&2 "Benchmarks:"
  ls ./benchmarks | sed 's/^/  - /'
  exit 1
fi

export SELF_IMAGE="$(../../nix/print-docker-image.sh)"

trap stop EXIT INT QUIT TERM
./start.sh

info 'Running the benchmarks'
docker compose run --rm benchmark run "/benchmarks/$BENCHMARK"
