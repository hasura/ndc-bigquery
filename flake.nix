{
  description = "PostgreSQL data connector";

  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs;
    crane.url = github:ipetkov/crane;
    crane.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = github:numtide/flake-utils;
  };

  outputs = { self, nixpkgs, crane, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = crane.lib.${system};
      in
    {
      packages.default = craneLib.buildPackage {
        src = craneLib.cleanCargoSource (craneLib.path ./.);
        doCheck = true;

        # Add extra inputs here or any other derivation settings
        # buildInputs = [];
        # nativeBuildInputs = [];
      };

      devShells.default = pkgs.mkShell {
        buildInputs = [
          # build
          pkgs.cargo
          pkgs.cargo-watch
          pkgs.clippy
          pkgs.rust-analyzer
          pkgs.rustPlatform.rustcSrc
          pkgs.rustc
          pkgs.rustfmt

          # runtime
          pkgs.protobuf

          # development
          pkgs.just
        ];
      };
    });
}
