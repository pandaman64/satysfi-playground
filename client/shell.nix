with import <nixpkgs> {};
stdenv.mkDerivation {
  inherit (import ./default.nix);
  name = "satysfi-playground";
  buildInputs = [
    # interactive bash
    bashInteractive

    # rust environment
    (latest.rustChannels.nightly.rust.override { targets = [ "wasm32-unknown-unknown" ]; })

    # openssl dev for cargo-web
    openssl
    pkgconfig

    # javascript envrionment
    nodePackages.npm
    nodejs
  ];
  shellHook = ''
    export PATH="$PWD/.cargo/bin:$PATH"
    if ! type wasm-bindgen > /dev/null 2> /dev/null; then
      cargo install wasm-bindgen-cli --root "$PWD/.cargo"
    fi
    cargo install cargo-web --root "$PWD/.cargo"
  '';
}

