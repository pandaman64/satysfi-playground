{
  description = "SATySFi Playground";

  inputs.nixpkgs.url = github:NixOS/nixpkgs/nixos-21.11;
  inputs.crate2nix = {
    url = github:kolloch/crate2nix;
    flake = false;
  };
  inputs.naersk.url = github:nix-community/naersk;

  outputs = { self, nixpkgs, crate2nix, naersk }:
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
      naersk-lib = naersk.lib.x86_64-linux;
    in
    {
      # packages.x86_64-linux.server-crate2nix = server.rootCrate.build;
      packages.x86_64-linux.server = naersk-lib.buildPackage ./server;
      packages.x86_64-linux.satysfi-docker = pkgs.callPackage ./satysfi-docker.nix { };

      nixosModules.satysfi-playground = import ./service.nix {
        server = self.packages.x86_64-linux.server;
        satysfi-docker = self.packages.x86_64-linux.satysfi-docker;
      };

      nixosConfigurations.satysfi-playground = import ./nixos-configuration.nix {
        inherit self nixpkgs;
        system = "x86_64-linux";
      };

      defaultPackage.x86_64-linux = self.packages.x86_64-linux.server;
      defaultApp.x86_64-linux = {
        type = "app";
        program = "${self.packages.x86_64-linux.server}/bin/server";
      };

      # TODO: format/lint Rust/Terraform/Nix, local snapshot testing (maybe)
      checks.x86_64-linux.satysfi-playground = pkgs.callPackage ./test.nix {
        system = "x86_64-linux";
        satysfi-playground = self.nixosModules.satysfi-playground;
      };

      devShell.x86_64-linux = pkgs.mkShell {
        buildInputs = [
          pkgs.rustup
          pkgs.cargo-edit
          pkgs.nixpkgs-fmt
          pkgs.jq
          pkgs.minio-client
          pkgs.curl
          pkgs.awscli2
          pkgs.terraform
        ];
      };
    };
}
