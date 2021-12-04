{ self, nixpkgs, system }:
nixpkgs.lib.nixosSystem {
  inherit system;

  modules = [
    ({ config, pkgs, ... }: {
      imports = [
        "${nixpkgs}/nixos/modules/virtualisation/amazon-image.nix"
      ];

      system.configurationRevision = nixpkgs.lib.mkIf (self ? rev) self.rev;

      networking.firewall.allowedTCPPorts = [ 22 80 ];

      services.nginx.enable = true;
    })
  ];
}
