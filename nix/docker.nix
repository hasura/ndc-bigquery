# This is a function that returns a derivation for a docker image.
{ postgres-agent
, dockerTools
, lib
, architecture ? null
, name ? "ghcr.io/hasura/postgres-agent-rs"

  # See config options at https://github.com/moby/moby/blob/master/image/spec/v1.2.md#image-json-field-descriptions
, extraConfig ? { }
}:

let
  args = {
    inherit name;
    created = "now";

    config = {
      Entrypoint = [
        "${postgres-agent}/bin/postgres-multitenant-ndc"
        "--deployments-dir"
        "/data/deployments"
      ];
      ExposedPorts = { "3000/tcp" = { }; };
      Env = [
        "PORT=3000"
      ];
      Volumes = {
        "/data/deployments/" = {};
      };
    } // extraConfig;
  }
  // lib.optionalAttrs (architecture != null) {
    inherit architecture;
  };
in
dockerTools.buildLayeredImage args
