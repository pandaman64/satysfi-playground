{ nixpkgs, system, nixosModule }:
let
  output = builtins.fromJSON (builtins.readFile ./terraform/output.json);
in
nixpkgs.lib.nixosSystem {
  inherit system;

  modules = [
    nixosModule
    ({ config, pkgs, ... }: {
      imports = [
        "${nixpkgs}/nixos/modules/virtualisation/amazon-image.nix"
      ];

      networking.firewall.allowedTCPPorts = [ 22 80 443 ];

      services.satysfi-playground = rec {
        enable = true;
        logLevel = "DEBUG";
        s3ApiEndpoint = "https://s3.${region}.amazonaws.com";
        s3PublicEndpoint = "https://${output.s3_public_domain_name.value}";
        region = output.s3_region.value;
      };
    })
  ];
}
