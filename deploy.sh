#!/usr/bin/env bash
# Run this script from the development shell.

set -euo pipefail

# Check if the test passes.
nix flake check

# Run terraform and retrieve public ip of EC2 instance.
(
    cd terraform
    terraform apply
)
PUBLIC_IP=$(
    cd terraform
    terraform output -raw public_ip
)

# Update the machine with the latest NixOS configuration.
nixos-rebuild switch --target-host "root@${PUBLIC_IP}" --flake '.#satysfi-playground'

# Done.
echo "${PUBLIC_IP}"