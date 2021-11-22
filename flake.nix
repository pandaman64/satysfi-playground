{
  description = "SATySFi Playground";

  inputs.nixpkgs.url = github:NixOS/nixpkgs/nixos-21.05;
  inputs.crate2nix = {
    url = github:kolloch/crate2nix;
    flake = false;
  };

  outputs = { self, nixpkgs, crate2nix }:
    let
      pkgs = nixpkgs.legacyPackages."x86_64-linux";
      inherit (import "${crate2nix}/tools.nix" { inherit pkgs; }) generatedCargoNix;
      server = (import
        (generatedCargoNix {
          name = "server";
          src = ./server;
        })
        {
          inherit pkgs;
          # For development speed. TODO: enable release build
          release = false;
        });
    in
    {
      packages.x86_64-linux.server = server.rootCrate.build;
      packages.x86_64-linux.satysfi-docker = pkgs.callPackage ./satysfi-docker.nix { };

      nixosModules.satysfi-playground = import ./service.nix {
        server = self.packages.x86_64-linux.server;
        satysfi-docker = self.packages.x86_64-linux.satysfi-docker;
      };

      defaultPackage.x86_64-linux = self.packages.x86_64-linux.server;
      defaultApp.x86_64-linux = {
        type = "app";
        program = "${self.packages.x86_64-linux.server}/bin/server";
      };

      checks.x86_64-linux.satysfi-playground = pkgs.callPackage ./test.nix {
        system = "x86_64-linux";
        satysfi-playground = self.nixosModules.satysfi-playground;
      };

      devShell.x86_64-linux = pkgs.mkShell {
        buildInputs = [
          pkgs.rustup
          pkgs.cargo-edit
          pkgs.nixpkgs-fmt
        ];
      };
    };
}
