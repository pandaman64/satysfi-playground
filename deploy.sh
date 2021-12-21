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

# Update Vercel environment variables
API_ENDPOINT_DOMAIN=$(jq --raw-output '.api_domain_name.value' "${BASEDIR}/terraform/output.json")
S3_PUBLIC_ENDPOINT_DOMAIN=$(jq --raw-output '.s3_public_domain_name.value' "${BASEDIR}/terraform/output.json")

for environment in production preview development
do
    # It's ok to fail removing environment variables
    echo y | vercel env rm API_ENDPOINT "${environment}" || true
    echo y | vercel env rm S3_PUBLIC_ENDPOINT "${environment}" || true

    echo "https://${API_ENDPOINT_DOMAIN}" | vercel env add API_ENDPOINT "${environment}"
    echo "https://${S3_PUBLIC_ENDPOINT_DOMAIN}" | vercel env add S3_PUBLIC_ENDPOINT "${environment}"
done

# Done.
echo "${PUBLIC_IP}"