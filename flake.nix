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
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      server =
        let
          inherit (import "${crate2nix}/tools.nix" { inherit pkgs; }) generatedCargoNix;
        in
        (import
          (generatedCargoNix {
            name = "server";
            src = ./server;
          })
          {
            inherit pkgs;
          });
      naersk-lib = naersk.lib.${system};
    in
    rec {
      packages.${system} = {
        # server-crate2nix = server.rootCrate.build;
        server = naersk-lib.buildPackage ./server;
        satysfi-docker = pkgs.callPackage ./satysfi-docker.nix { };
      };

      # TODO: figure out how to abstact out systems when we define nixosModules/nixosConfiguraions
      nixosModules.satysfi-playground = import ./service.nix {
        server = packages.${system}.server;
        satysfi-docker = packages.${system}.satysfi-docker;
      };

      nixosConfigurations.satysfi-playground = import ./nixos-configuration.nix {
        inherit nixpkgs system;
      };

      defaultPackage.${system} = packages.${system}.server;
      defaultApp.${system} = {
        type = "app";
        program = "${packages.${system}.server}/bin/server";
      };

      # TODO: format/lint Rust/Terraform/Nix, local snapshot testing (maybe)
      checks.${system} = {
        satysfi-playground = pkgs.callPackage ./test.nix {
          inherit system;
          satysfi-playground = nixosModules.satysfi-playground;
        };
      };

      devShell.${system} = pkgs.mkShell {
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
