# NixOS tests for SATySFi Playground
{ pkgs, system, satysfi-playground, ... }:
let
  region = "ap-northeast-1";
  accessKeyId = "minio-access-key";
  secretAccessKey = "minio-secret-key";
in
pkgs.nixosTest {
  inherit system;

  nodes = {
    server = { config, pkgs, ... }: {
      imports = [ satysfi-playground ];

      networking.firewall = {
        enable = true;
        allowedTCPPorts = [ 8080 ];
      };

      services.satysfi-playground = {
        enable = true;
        logLevel = "DEBUG";
        inherit accessKeyId secretAccessKey region;
      };

      services.minio = {
        enable = true;
        accessKey = accessKeyId;
        secretKey = secretAccessKey;
        inherit region;
      };

      systemd.services.create-bucket = {
        description = ''Create S3 buckets neccessary for SATySFi Playground'';
        after = [ "minio.service" ];
        before = [ "satysfi-playground.service" ];
        requiredBy = [ "satysfi-playground.service" ];

        serviceConfig = {
          Type = "oneshot";
          ExecStart = pkgs.writeShellScript "create-bucket" ''
            ${pkgs.awscli2}/bin/aws --endpoint-url http://localhost:9000 s3 mb s3://satysfi-playground
          '';
          Environment = [
            "AWS_ACCESS_KEY_ID=${accessKeyId}"
            "AWS_SECRET_ACCESS_KEY=${secretAccessKey}"
            "AWS_DEFAULT_REGION=${region}"
          ];
        };
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
      with open(os.path.join(entry.path, "input.saty"), "rb") as f:
        input = f.read()
        request = json.dumps({
          "source": base64.b64encode(input).decode("ascii"),
        })

        with subtest(f"Compile {entry.name}"):
          response = json.loads(client.succeed(
            f"${pkgs.curl}/bin/curl -d '{request}' -H 'Content-Type: application/json' -f http://server:8080/compile"
          ))
          response["stdout"] = base64.b64decode(response["stdout"].encode("ascii")).decode("ascii")
          response["stderr"] = base64.b64decode(response["stderr"].encode("ascii")).decode("ascii")
          assert response["status"] == 0, response

        with subtest(f"Persist {entry.name}"):
          # Request to /persist must succeed
          response = json.loads(client.succeed(
            f"${pkgs.curl}/bin/curl -d '{request}' -H 'Content-Type: application/json' -f http://server:8080/persist"
          ))
          assert response["status"] == 0, response

          # All files are stored in S3
          # TODO: enable this test
          # client.succeed(f"""${pkgs.curl}/bin/curl -f '{response["s3_url"]}/stdout.txt'""")
          # client.succeed(f"""${pkgs.curl}/bin/curl -f '{response["s3_url"]}/stderr.txt'""")
          # client.succeed(f"""${pkgs.curl}/bin/curl -f '{response["s3_url"]}/document.pdf'""")
  '';
}
