# A NixOS module for the satysfi-playground service
# genPackages: system -> { server, satysfi-docker }
genPackages:
{ config, lib, pkgs, ... }:
let
  inherit (genPackages pkgs.system) server satysfi-docker;
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
    # nginx reverse proxy
    {
      security.acme = {
        email = "pandaman64+acme@gmail.com";
        acceptTerms = true;
      };

      services.nginx = {
        enable = true;

        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;

        virtualHosts."api.satysfi-playground.tech" = {
          enableACME = true;
          forceSSL = true;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8080";
          };
        };
      };
    }
    # SATySFi Playground service
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
          # TODO: I could not make newuidmap work with systemd services. some logs will be shown in stderr...
          # subUidRanges = [
          #   {
          #     startUid = 100000;
          #     count = 65536;
          #   }
          # ];
          # subGidRanges = [
          #   {
          #     startGid = 1000000;
          #     count = 65536;
          #   }
          # ];
        };
      };

      # Oneshot unit for loading SATySFi Docker image into Podman
      systemd.services.load-satysfi-docker = {
        description = "SATySFi Playground Docker Image Loader";
        # for newuidmap/newgidmap used by podman
        # path = [ pkgs.shadow ];

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
        # for newuidmap/newgidmap used by podman
        # path = [ pkgs.shadow ];

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

          # Security and Sandboxing settings
          PrivateMounts = true;
          PrivateUsers = true;
          PrivateTmp = true;
          # The following options must be disabled to run podman.
          # PrivateDevices = true;
          # PrivateNetwork = true;

          CapabilityBoundingSet = "";

          RestrictAddressFamilies = [
            # We need internet access for S3.
            # Note: incoming request can use systemd's socket activation.
            "AF_INET"
            "AF_INET6"
            # Needed for journald
            "AF_UNIX"
          ];

          # Podman relies on user namespaces.
          RestrictNamespaces = false;
          RestrictRealtime = true;
          RestrictSUIDSGID = true;

          NoNewPrivileges = true;
          ProtectKernelLogs = true;
          # Somehow this does not work
          ProtectKernelModuels = true;
          ProtectKernelTunables = true;
          ProtectProc = "noaccess";

          # According to systemd-analyze security
          SystemCallArchitectures = "native";
          # TODO: SystemCallFilter

          MemoryDenyWriteExecute = true;

          RemoveIPC = true;
        };
      };
    }
  ]);
}
