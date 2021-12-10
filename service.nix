# A NixOS module for the satysfi-playground service
{ server, satysfi-docker }:
{ config, lib, pkgs, ... }:
let
  cfg = config.services.satysfi-playground;
  podman = config.virtualisation.podman.package;
  user = "satysfi-playground";
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

    logLevel = lib.mkOption {
      type = lib.types.str;
      default = "";
      description = ''
        RUST_LOG
      '';
    };

    s3ApiEndpoint = lib.mkOption {
      type = lib.types.str;
      description = ''
        The URL of the S3 REST API endpoint
      '';
    };

    s3PublicEndpoint = lib.mkOption {
      type = lib.types.str;
      description = ''
        The URL of the S3 Public endpoint
      '';
    };

    accessKeyId = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = ''
        AWS_ACCESS_KEY_ID
      '';
    };

    secretAccessKey = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = ''
        AWS_SECRET_KEY
      '';
    };

    region = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = ''
        AWS_DEFAULT_REGION
      '';
    };
  };

  config = lib.mkIf cfg.enable (lib.mkMerge [
    {
      virtualisation.podman.enable = true;

      users = {
        groups.${user} = { };
        users.${user} = {
          group = user;
          isSystemUser = true;
          # Podman writes to $HOME/.local/share/containers/storage
          createHome = true;
          home = "/home/${user}";
        };
      };

      # Oneshot unit for loading SATySFi Docker image into Podman
      systemd.services.load-satysfi-docker = {
        description = "SATySFi Playground Docker Image Loader";

        serviceConfig = {
          Type = "oneshot";
          ExecStart = "${podman}/bin/podman load -i ${satysfi-docker}";
          User = user;
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
          Environment = lib.mkMerge
            [
              [
                "RUST_LOG=${cfg.logLevel}"
                "PODMAN=${podman}/bin/podman"
                "SATYSFI_DOCKER_VERSION=${satysfi-docker}"
                "S3_API_ENDPOINT=${cfg.s3ApiEndpoint}"
                "S3_PUBLIC_ENDPOINT=${cfg.s3PublicEndpoint}"
              ]
              (lib.mkIf (cfg.accessKeyId != null) [
                "AWS_ACCESS_KEY_ID=${cfg.accessKeyId}"
              ])
              (lib.mkIf (cfg.secretAccessKey != null) [
                "AWS_SECRET_ACCESS_KEY=${cfg.secretAccessKey}"
              ])
              (lib.mkIf (cfg.region != null) [
                "AWS_DEFAULT_REGION=${cfg.region}"
              ])
            ];

          User = user;
        };
      };
    }
  ]);
}
