{ pkgs }:
pkgs.dockerTools.buildImage {
  name = "satysfi";
  tag = "latest";

  contents = pkgs.satysfi;

  config = {
    Cmd = [ "/bin/satysfi" ];
  };
}
