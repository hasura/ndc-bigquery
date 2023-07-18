# Component benchmarks

This is the benchmark suite for the PostgreSQL data connector.

Running `run.sh` will:

1. build the PostgreSQL data connector Docker image,
2. start the database with Chinook data,
3. start the agent using an associated deployment, and
4. run the benchmarks using k6.

Everything is run through Docker Compose.

The Docker image for the agent is built with Nix. If you haven't built with Nix
before (or it's been a while), this may take some time at first.

## Requirements

1. _Nix_, to build the Docker image
   1. Install [Nix](https://nixos.org/download.html)
   2. Configure Nix by adding the following line to `~/.config/nix/nix.conf`:
      ```
      extra-experimental-features = flakes nix-command
      ```
2. _Docker_ and _Docker Compose_, to run the containers (see the root README)

## Viewing the benchmark results

When the benchmarks finish, the results will be printed.

There is a Grafana dashboard which can be viewed as follows:

1. Open [http://localhost:64300][].
2. Open the menu on the left and choose "Dashboards".
3. Choose the "Test Result" dashboard.
