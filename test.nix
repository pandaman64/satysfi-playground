# NixOS tests for SATySFi Playground
{ pkgs, lib, system, satysfi-playground, ... }:
let
  region = "ap-northeast-1";
  accessKeyId = "minio-access-key";
  secretAccessKey = "minio-secret-key";
  # Minio Client seems to need getent in path
  mc = pkgs.minio-client.overrideAttrs (oldAttrs: {
    buildInputs = (oldAttrs.buildInputs or [ ]) ++ [ pkgs.makeWrapper ];
    postInstall = (oldAttrs.postInstall or "") + ''
      wrapProgram "$out/bin/mc" --prefix PATH : ${lib.makeBinPath [ pkgs.getent ]}
    '';
  });
in
pkgs.nixosTest {
  inherit system;

  nodes = {
    server = { config, pkgs, ... }: {
      # In megabytes. The uncompressed Docker image amounts to >500MB.
      virtualisation.diskSize = 4096;

      imports = [ satysfi-playground ];

      networking.firewall = {
        enable = true;
        allowedTCPPorts = [ 8080 9000 ];
      };

      services.satysfi-playground = {
        enable = true;
        logLevel = "DEBUG";
        # I could'nt successfully run tests with virtual hosted-style buckets. So we use path-style buckets here.
        s3ApiEndpoint = "http://server:9000";
        s3PublicEndpoint = "http://server:9000/satysfi-playground";
        inherit accessKeyId secretAccessKey region;
      };

      services.minio = {
        enable = true;
        listenAddress = "0.0.0.0:9000";
        rootCredentialsFile = pkgs.writeText "minio-credentials" ''
          MINIO_ROOT_USER=${accessKeyId}
          MINIO_ROOT_PASSWORD=${secretAccessKey}
        '';
        inherit region;
      };

      systemd.services.create-bucket = {
        description = ''Create Minio buckets neccessary for SATySFi Playground'';
        after = [ "minio.service" ];
        before = [ "satysfi-playground.service" ];
        requiredBy = [ "satysfi-playground.service" ];
        serviceConfig = {
          Type = "oneshot";
          TimeoutStartSec = "10s";
        };
        script = ''
          set -euo pipefail

          # Minio requires a writable directory for configurations
          CONFIG_DIR=$(mktemp -d)

          # Loop until `mc alias` succeeds because it fails if Minio server has not started yet.
          while ! ${mc}/bin/mc -C "$CONFIG_DIR" alias set local http://localhost:9000 ${accessKeyId} ${secretAccessKey}
          do
            sleep 1
          done
          ${mc}/bin/mc -C "$CONFIG_DIR" mb --region='${region}' local/satysfi-playground
          # The access policy is a Minio-specific part.
          # If we use S3 in production, we need to figure out a way to allow public access with S3.
          ${mc}/bin/mc -C "$CONFIG_DIR" policy set download local/satysfi-playground
        '';
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

          # All files are stored in S3.
          # `-o /dev/null` is required because otherwise curl returns error code 23 if the content is a binary.
          client.succeed(f"""${pkgs.curl}/bin/curl -fs -o /dev/null '{response["s3_url"]}/stdout.txt'""")
          client.succeed(f"""${pkgs.curl}/bin/curl -fs -o /dev/null '{response["s3_url"]}/stderr.txt'""")
          client.succeed(f"""${pkgs.curl}/bin/curl -fs -o /dev/null '{response["s3_url"]}/document.pdf'""")
  '';
}
