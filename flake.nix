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
      api = import
        (generatedCargoNix {
          name = "api";
          src = ./api;
        })
        {
          inherit pkgs;
        };
    in
    {
      packages.x86_64-linux.api = api.rootCrate.build;
      packages.x86_64-linux.satysfi-docker = pkgs.callPackage (import ./satysfi-docker.nix) { };
      defaultPackage.x86_64-linux = self.packages.x86_64-linux.api;
      devShell.x86_64-linux = pkgs.mkShell {
        buildInputs = [
          pkgs.rustup
          pkgs.cargo-edit
          pkgs.nixpkgs-fmt
        ];
      };
    };
}
