{ nixpkgs, system, nixosModule }:
nixpkgs.lib.nixosSystem {
  inherit system;

  modules = [
    nixosModule
    ({ config, pkgs, ... }: {
      imports = [
        "${nixpkgs}/nixos/modules/virtualisation/amazon-image.nix"
      ];

      networking.firewall.allowedTCPPorts = [ 22 8080 ];

      services.satysfi-playground = rec {
        enable = true;
        logLevel = "DEBUG";
        s3ApiEndpoint = "https://s3.${region}.amazonaws.com";
        s3PublicEndpoint = "https://satysfi-playground.s3.amazonaws.com";
        region = "ap-northeast-1";
      };
    })
  ];
}
