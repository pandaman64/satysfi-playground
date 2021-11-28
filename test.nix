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

      services.minio = {
        enable = true;
        accessKey = "minio-access-key";
        secretKey = "minio-secret-key";
        region = "ap-northeast-1";
      };
    };

    client = { ... }: { };
  };

  testScript = ''
    import base64
    import json
    import os
    import os.path

    start_all()
    server.wait_for_unit("satysfi-playground.service")
    server.wait_for_open_port(8080)

    with subtest("Healthcheck succeeds"):
      response = client.succeed(
        "${pkgs.curl}/bin/curl http://server:8080/healthcheck"
      )
      assert response == "Hello, World!"

    for entry in os.scandir("${./examples}"):
      with subtest(f"Compile {entry.name}"):
        with open(os.path.join(entry.path, "input.saty"), "rb") as f:
          input = f.read()
          request = json.dumps({
            "source": base64.b64encode(input).decode("ascii"),
          })
          response = json.loads(client.succeed(
            f"${pkgs.curl}/bin/curl -d '{request}' -H 'Content-Type: application/json' -f http://server:8080/compile"
          ))
          response["stdout"] = base64.b64decode(response["stdout"].encode("ascii")).decode("ascii")
          response["stderr"] = base64.b64decode(response["stderr"].encode("ascii")).decode("ascii")
          assert response["status"] == 0, response
  '';
}
