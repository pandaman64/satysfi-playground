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

      nixosModules.satysfi-playground = import ./service.nix (system: {
        server = packages.${system}.server;
        satysfi-docker = packages.${system}.satysfi-docker;
      });

      # TODO: figure out how to abstact out systems when we define nixosConfiguraions
      nixosConfigurations.satysfi-playground = import ./nixos-configuration.nix {
        inherit nixpkgs system;
        nixosModule = nixosModules.satysfi-playground;
      };

      defaultPackage.${system} = packages.${system}.server;
      defaultApp.${system} = {
        type = "app";
        program = "${packages.${system}.server}/bin/server";
      };

      # TODO: Rust checks should be done inside the server derivation
      # TODO: frontend lints cannot run without node_modules
      checks.${system} = {
        # TODO: Add tflint
        terraform = pkgs.runCommand "terraform-check"
          {
            buildInputs = [ pkgs.terraform ];
          } ''
          set -euo pipefail

          cd "${./terraform}"
          terraform fmt -check

          touch $out
        '';
        nix = pkgs.runCommand "nix-check"
          {
            buildInputs = [ pkgs.nixpkgs-fmt ];
          } ''
          set -euo pipefail

          cd "${./.}"
          nixpkgs-fmt --check *.nix

          touch $out
        '';
        bash = pkgs.runCommand "bash-check"
          {
            buildInputs = [ pkgs.shellcheck ];
          } ''
          set -euo pipefail

          cd "${./.}"
          shellcheck -o all *.sh

          touch $out
        '';
        satysfi-playground = pkgs.callPackage ./test.nix {
          inherit system;
          satysfi-playground = nixosModules.satysfi-playground;
        };
      };

      devShell.${system} = pkgs.mkShell {
        buildInputs = [
          pkgs.rustup
          pkgs.cargo-edit
          pkgs.nodejs
          pkgs.nixpkgs-fmt
          pkgs.jq
          pkgs.ncat
          pkgs.minio-client
          pkgs.curl
          # awscli2 pollutes PYTHONPATH, which makes nixos-test-driver unrunnable.
          # https://github.com/NixOS/nixpkgs/issues/47900
          (
            pkgs.writeShellScriptBin "aws" ''
              unset PYTHONPATH
              exec ${pkgs.awscli2}/bin/aws "$@"
            ''
          )
          pkgs.terraform
          pkgs.nixos-option
          pkgs.shellcheck
          pkgs.nodePackages.vercel
        ];
      };
    };
}
