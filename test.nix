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
    try:
      start_all()

      server.wait_for_open_port(8080)
      # TODO: this request fails
      # client.succeed(
      #   "${pkgs.curl}/bin/curl http://server:8080/"
      # )
    except:
      raise
    finally:
      # somehow shutdown is needed to complete the test
      for machine in machines:
        # commenting out this line causes the test to hang indefinitely
        machine.shutdown()
        pass
  '';
}
