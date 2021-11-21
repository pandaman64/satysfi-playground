{
  description = "SATySFi Playground";

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
        });
    in
    {
      packages.x86_64-linux.server = server.rootCrate.build;
      packages.x86_64-linux.satysfi-docker = pkgs.callPackage (import ./satysfi-docker.nix) { };

      defaultPackage.x86_64-linux = self.packages.x86_64-linux.server;
      defaultApp.x86_64-linux = {
        type = "app";
        program = "${self.packages.x86_64-linux.server}/bin/server";
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
