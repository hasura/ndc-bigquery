{
  description = "PostgreSQL data connector";

  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs/nixos-unstable;
    flake-utils.url = github:numtide/flake-utils;

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
      inputs.flake-utils.follows = "flake-utils";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, rust-overlay, advisory-db }:
    flake-utils.lib.eachDefaultSystem (localSystem:
      let
        pkgs = import nixpkgs {
          system = localSystem;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Edit ./nix/ndc-agent.nix to adjust library and buildtime
        # dependencies or other build configuration for postgres-agent
        crateExpression = import ./nix/ndc-agent.nix;

        # cockroachExpression = import ./nix/cockroach-agent.nix;
        cargoBuild = import ./nix/cargo-build.nix;

        ### POSTGRES ###

        # Build for the architecture and OS that is running the build
        postgres-agent = cargoBuild {
          binary-name = "ndc-postgres";
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
        };

        inherit (postgres-agent) cargoArtifacts rustToolchain craneLib buildArgs;

        postgres-agent-x86_64-linux = cargoBuild {
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
          binary-name = "ndc-postgres";
          crossSystem = "x86_64-linux";
        };

        postgres-agent-aarch64-linux = cargoBuild {
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
          binary-name = "ndc-postgres";
          crossSystem = "aarch64-linux";
        };

        ### COCKROACH ###

        # Build for the architecture and OS that is running the build
        cockroach-agent = cargoBuild {
          binary-name = "ndc-cockroach";
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
        };

        cockroach-agent-x86_64-linux = cargoBuild {
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
          binary-name = "ndc-cockroach";
          crossSystem = "x86_64-linux";
        };

        cockroach-agent-aarch64-linux = cargoBuild {
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
          binary-name = "ndc-cockroach";
          crossSystem = "aarch64-linux";
        };

        ### CITUS ###

        # Build for the architecture and OS that is running the build
        citus-agent = cargoBuild {
          binary-name = "ndc-citus";
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
        };

        citus-agent-x86_64-linux = cargoBuild {
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
          binary-name = "ndc-citus";
          crossSystem = "x86_64-linux";
        };

        citus-agent-aarch64-linux = cargoBuild {
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
          binary-name = "ndc-citus";
          crossSystem = "aarch64-linux";
        };

        ### AURORA ###

        # Build for the architecture and OS that is running the build
        aurora-agent = cargoBuild {
          binary-name = "ndc-aurora";
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
        };

        aurora-agent-x86_64-linux = cargoBuild {
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
          binary-name = "ndc-aurora";
          crossSystem = "x86_64-linux";
        };

        aurora-agent-aarch64-linux = cargoBuild {
          inherit crateExpression nixpkgs crane rust-overlay localSystem;
          binary-name = "ndc-aurora";
          crossSystem = "aarch64-linux";
        };


      in
      {
        checks = {
          # Build the crate as part of `nix flake check`
          inherit postgres-agent;

          crate-clippy = craneLib.cargoClippy (buildArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          crate-nextest = craneLib.cargoNextest (buildArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });

          crate-audit = craneLib.cargoAudit {
            inherit advisory-db;
            inherit (postgres-agent) src;
          };
        };

        packages = {
          default = postgres-agent;

          docker = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = postgres-agent;
            binary-name = "ndc-postgres";
            image-name = "ghcr.io/hasura/postgres-agent-rs";
          };

          dockerDev = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = postgres-agent;
            binary-name = "ndc-postgres";
            image-name = "ghcr.io/hasura/postgres-agent-rs";
            tag = "dev";
          };

          /* postgres ndc targets */
          postgres-agent = postgres-agent;
          postgres-agent-x86_64-linux = postgres-agent-x86_64-linux;
          postgres-agent-aarch64-linux = postgres-agent-aarch64-linux;

          docker-postgres-x86_64-linux = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = postgres-agent-x86_64-linux;
            architecture = "amd64";
            binary-name = "ndc-postgres";
            image-name = "ghcr.io/hasura/postgres-agent-rs";
          };

          docker-postgres-aarch64-linux = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = postgres-agent-aarch64-linux;
            architecture = "arm64";
            binary-name = "ndc-postgres";
            image-name = "ghcr.io/hasura/postgres-agent-rs";
          };

          /* cockroach ndc targets */
          cockroach-agent = cockroach-agent;
          cockroach-agent-x86_64-linux = cockroach-agent-x86_64-linux;
          cockroach-agent-aarch64-linux = cockroach-agent-aarch64-linux;

          /* build Docker for Cockroach with whatever the local dev env is */
          docker-cockroach-dev = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = cockroach-agent;
            binary-name = "ndc-cockroach";
            image-name = "ghcr.io/hasura/cockroach-agent-rs";
            tag = "dev";
          };

          docker-cockroach-x86_64-linux = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = cockroach-agent-x86_64-linux;
            architecture = "amd64";
            binary-name = "ndc-cockroach";
            image-name = "ghcr.io/hasura/cockroach-agent-rs";
          };

          docker-cockroach-aarch64-linux = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = cockroach-agent-aarch64-linux;
            architecture = "arm64";
            binary-name = "ndc-cockroach";
            image-name = "ghcr.io/hasura/cockroach-agent-rs";
          };

          /* citus ndc targets */
          citus-agent = citus-agent;
          citus-agent-x86_64-linux = citus-agent-x86_64-linux;
          citus-agent-aarch64-linux = citus-agent-aarch64-linux;

          /* build Docker for Citus with whatever the local dev env is */
          docker-citus-dev = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = citus-agent;
            binary-name = "ndc-citus";
            image-name = "ghcr.io/hasura/citus-agent-rs";
            tag = "dev";
          };

          docker-citus-x86_64-linux = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = citus-agent-x86_64-linux;
            architecture = "amd64";
            binary-name = "ndc-citus";
            image-name = "ghcr.io/hasura/citus-agent-rs";
          };

          docker-citus-aarch64-linux = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = citus-agent-aarch64-linux;
            architecture = "arm64";
            binary-name = "ndc-citus";
            image-name = "ghcr.io/hasura/citus-agent-rs";
          };

          /* aurora ndc targets */
          aurora-agent = aurora-agent;
          aurora-agent-x86_64-linux = aurora-agent-x86_64-linux;
          aurora-agent-aarch64-linux = aurora-agent-aarch64-linux;

          /* build Docker for Citus with whatever the local dev env is */
          docker-aurora-dev = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = aurora-agent;
            binary-name = "ndc-aurora";
            image-name = "ghcr.io/hasura/aurora-agent-rs";
            tag = "dev";
          };

          docker-aurora-x86_64-linux = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = aurora-agent-x86_64-linux;
            architecture = "amd64";
            binary-name = "ndc-aurora";
            image-name = "ghcr.io/hasura/aurora-agent-rs";
          };

          docker-aurora-aarch64-linux = pkgs.callPackage ./nix/docker.nix {
            ndc-agent = aurora-agent-aarch64-linux;
            architecture = "arm64";
            binary-name = "ndc-aurora";
            image-name = "ghcr.io/hasura/aurora-agent-rs";
          };


          publish-docker-image = pkgs.writeShellApplication {
            name = "publish-docker-image";
            runtimeInputs = with pkgs; [ coreutils skopeo ];
            text = builtins.readFile ./ci/deploy.sh;
          };
        };

        formatter = pkgs.nixpkgs-fmt;

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks.${localSystem};
          nativeBuildInputs = [
            # runtime
            pkgs.protobuf

            # development
            pkgs.cargo-edit
            pkgs.cargo-flamegraph
            pkgs.cargo-insta
            pkgs.cargo-machete
            pkgs.cargo-watch
            pkgs.just
            pkgs.k6
            pkgs.pkg-config
            pkgs.rnix-lsp
            rustToolchain
          ];
        };
      });
}
