# This is a function that returns a derivation for a docker image.
{ postgres-agent
, dockerTools
, lib
, architecture ? null
, name ? "ghcr.io/hasura/postgres-agent-rs"
, tag ? null # defaults to the output hash
, extraConfig ? { } # see config options at: https://github.com/moby/moby/blob/master/image/spec/v1.2.md#image-json-field-descriptions
}:

let
  args = {
    inherit name;
    created = "now";

    config = {
      Entrypoint = [
        "${postgres-agent}/bin/ndc-postgres"
      ];
      ExposedPorts = { "8100/tcp" = { }; };
    } // extraConfig;
  }
  // lib.optionalAttrs (tag != null) {
    inherit tag;
  } // lib.optionalAttrs (architecture != null) {
    inherit architecture;
  };
in
dockerTools.buildLayeredImage args
