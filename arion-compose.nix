let
  region = "ap-northeast-1";
  accessKeyId = "minio-access-key";
  secretAccessKey = "minio-secret-key";
in
{
  services.minio = { pkgs, ... }: {
    nixos.useSystemd = true;
    nixos.configuration.boot.tmpOnTmpfs = true;
    service.useHostStore = true;
    service.capabilities.SYS_ADMIN = true;

    service.ports = [
      # API
      "9000:9000"
      # Console
      "9001:9001"
    ];
    nixos.configuration.services = {
      minio = {
        enable = true;
        listenAddress = "0.0.0.0:9000";
        consoleAddress = "0.0.0.0:9001";
        rootCredentialsFile = pkgs.writeText "minio-credentials" ''
          MINIO_ROOT_USER=${accessKeyId}
          MINIO_ROOT_PASSWORD=${secretAccessKey}
        '';
        inherit region;
      };
    };
  };
  services.satysfi-playground = { pkgs, satysfi-playground-module, ... }: {
    nixos.useSystemd = true;
    nixos.configuration.boot.tmpOnTmpfs = true;
    service.useHostStore = true;
    service.capabilities.SYS_ADMIN = true;

    # How to use service in my NixOS module (satysfi-p)
    nixos.configuration.services.satysfi-playground = {
      enable = true;
      logLevel = "DEBUG";
      # relies on port forwarding
      s3ApiEndpoint = "http://localhost:9000";
      s3PublicEndpoint = "http://localhost:9000/satysfi-playground";
      inherit accessKeyId secretAccessKey region;
    };

    nixos.configuration.services.nginx.enable = true;
    nixos.configuration.services.nginx.virtualHosts.localhost.root = "${pkgs.nix.doc}/share/doc/nix/manual";
    service.ports = [
      "8000:80" # host:container
    ];
  };
}
