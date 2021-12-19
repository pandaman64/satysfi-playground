#!/usr/bin/env bash
# Run this script from the development shell.

set -euo pipefail

BASEDIR=$(dirname -- "${BASH_SOURCE[0]}")

# Run terraform and retrieve public ip of EC2 instance.
terraform -chdir="${BASEDIR}/terraform" apply
terraform -chdir="${BASEDIR}/terraform" output -json > "${BASEDIR}/terraform/output.json"

# Check if the test passes after touching terraform/output.json.
nix flake check

# Update the machine with the latest NixOS configuration.
PUBLIC_IP=$(jq --raw-output '.public_ip.value' "${BASEDIR}/terraform/output.json")
nixos-rebuild switch --target-host "root@${PUBLIC_IP}" --flake '.#satysfi-playground'

# Done.
echo "${PUBLIC_IP}"