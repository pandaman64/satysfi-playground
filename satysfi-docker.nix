{ pkgs }:
pkgs.dockerTools.buildImage {
  name = "satysfi";
  tag = "latest";

  contents = pkgs.satysfi;

  config = {
    Entrypoint = [ "/bin/satysfi" "-b" "-o" "/tmp/output.pdf" "/tmp/input.saty" ];
  };
}
