# A NixOS module for the satysfi-playground service
{ server, satysfi-docker }:
{ config, lib, pkgs, ... }:
let
  cfg = config.services.satysfi-playground;
  podman = config.virtualisation.podman.package;
in
{
  options.services.satysfi-playground = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Whether to enable the SATySFi Playground web service daemon.
      '';
    };
  };

  config = {
    # In megabytes. The uncompressed Docker image amounts to >500MB.
    virtualisation.diskSize = 4096;
    virtualisation.podman.enable = true;

    # Oneshot unit for loading SATySFi Docker image into Podman
    systemd.services.load-satysfi-docker = {
      description = "SATySFi Playground Docker Image Loader";

      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${podman}/bin/podman load -i ${satysfi-docker}";
      };
    };

    systemd.services.satysfi-playground = {
      description = "SATySFi Playground";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" "load-satysfi-docker.service" ];
      requires = [ "load-satysfi-docker.service" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${server}/bin/server";
        Environment = [
          "RUST_LOG=debug"
          "PODMAN=${podman}/bin/podman"
        ];
      };
    };
  };
}
