# A NixOS module for the satysfi-playground service
{ config, lib, pkgs, server, ... }:
let
  cfg = config.services.satysfi-playground;
in
{
  options = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Whether to enable the SATySFi Playground web service daemon.
      '';
    };
  };

  config = {
    systemd.services.satysfi-playground = {
      description = "SATySFi Playground";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${server}/bin/server";
      };
    };
  };
}
