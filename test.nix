# NixOS tests for SATySFi Playground
{ pkgs, system, satysfi-playground, ... }:
pkgs.nixosTest {
  inherit system;

  nodes = {
    server = { config, pkgs, ... }: {
      imports = [ satysfi-playground ];

      networking.firewall = {
        enable = true;
        allowedTCPPorts = [ 8080 ];
      };

      services.satysfi-playground.enable = true;
    };

    client = { ... }: { };
  };

  testScript = ''
    start_all()

    server.wait_for_open_port(8080)
    # TODO: this request fails
    # client.succeed(
    #   "${pkgs.curl}/bin/curl http://server:8080/"
    # )
  '';
}
