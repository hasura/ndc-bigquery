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

        # create binaries for a given NDC
        make-binaries = (binary-name: {
          inherit binary-name;
          # a binary for whichever is the local computer
          local-system = cargoBuild {
            inherit binary-name crateExpression nixpkgs crane rust-overlay localSystem;
          };
          # cross compiler an x86_64 linux binary
          x86_64-linux = cargoBuild {
            inherit binary-name crateExpression nixpkgs crane rust-overlay localSystem;
            crossSystem = "x86_64-linux";
          };
          # cross compile a aarch64 linux binary
          aarch64-linux = cargoBuild {
            inherit binary-name crateExpression nixpkgs crane rust-overlay localSystem;
            crossSystem = "aarch64-linux";
          };
        });

        # given the binaries, return the flake targets that build Docker etc
        make-packages =
          (ndc-binaries:
            let name = ndc-binaries.binary-name; in {
              # binary compiled on local system
              "${name}" = ndc-binaries.local-system;
              # binary compiled for x86_64-linux
              "${name}-x86_64-linux" = ndc-binaries.x86_64-linux;
              # binary compiled for aarch64-linux
              "${name}-aarch64-linux" = ndc-binaries.aarch64-linux;
              # docker for local system
              "${name}-docker" = pkgs.callPackage ./nix/docker.nix {
                ndc-agent = ndc-binaries.local-system;
                binary-name = name;
                image-name = "ghcr.io/hasura/${name}";
                tag = "dev";
              };
              # docker for x86_64-linux
              "${name}-docker-x86_64-linux" = pkgs.callPackage ./nix/docker.nix {
                ndc-agent = ndc-binaries.x86_64-linux;
                architecture = "amd64";
                binary-name = name;
                image-name = "ghcr.io/hasura/${name}";
              };
              # docker for aarch64-linux
              "${name}-docker-aarch64-linux" = pkgs.callPackage ./nix/docker.nix {
                ndc-agent = ndc-binaries.aarch64-linux;
                architecture = "arm64";
                binary-name = name;
                image-name = "ghcr.io/hasura/${name}";
              };
            });

        postgres-binaries = make-binaries "ndc-postgres";
        cockroach-binaries = make-binaries "ndc-cockroach";
        citus-binaries = make-binaries "ndc-citus";
        aurora-binaries = make-binaries "ndc-aurora";

        inherit (postgres-binaries.local-system) cargoArtifacts rustToolchain craneLib buildArgs;

      in
      {
        packages = builtins.foldl' (x: y: x // y) { } [
          (make-packages postgres-binaries)
          (make-packages cockroach-binaries)
          (make-packages citus-binaries)
          (make-packages aurora-binaries)
        ] // {
          default = postgres-binaries.local-system;

          publish-docker-image = pkgs.writeShellApplication {
            name = "publish-docker-image";
            runtimeInputs = with pkgs; [ coreutils skopeo ];
            text = builtins.readFile ./ci/deploy.sh;
          };
        };

        checks = {
          # Build the crate as part of `nix flake check`
          ndc-postgres = postgres-binaries.local-system;

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
            inherit (postgres-binaries.local-system) src;
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
            pkgs.nixpkgs-fmt
            pkgs.nodePackages.prettier
            pkgs.pkg-config
            pkgs.rnix-lsp
            rustToolchain
          ];
        };
      });
}
