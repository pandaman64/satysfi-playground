#!/usr/bin/env bash
# Run this script from the development shell.
# This script runs the backend services

# We do not have -e since we do not want to quit the script without waiting all children
set -uo pipefail

export AWS_ACCESS_KEY_ID=minioadmin
export AWS_SECRET_ACCESS_KEY=minioadmin
export AWS_DEFAULT_REGION='ap-northeast-1'
export MINIO_ROOT_USER="${AWS_ACCESS_KEY_ID}"
export MINIO_ROOT_PASSWORD="${AWS_SECRET_ACCESS_KEY}"
export S3_API_ENDPOINT='http://localhost:9000'
export S3_PUBLIC_ENDPOINT='http://localhost:9000/satysfi-playground'
export RUST_LOG='DEBUG'

# Object storage
podman run --rm -p 9000:9000 -p 9001:9001 \
    quay.io/minio/minio server /data --console-address ':9001' &

# API
nix run &

# Frontend server
(cd "$(dirname -- "${BASH_SOURCE[0]}")/frontend" || exit; npm run dev) &

# Set up buckets
while ! ncat -v localhost 9000 < /dev/null
do
    echo 'Waiting for localhost:9000...'
    sleep 1
done
CONFIG_DIR=$(mktemp -d)
while ! mc -C "${CONFIG_DIR}" alias set local http://localhost:9000 "${MINIO_ROOT_USER}" "${MINIO_ROOT_PASSWORD}"
do
    echo 'Waiting for minio...'
    sleep 1
done
mc -C "${CONFIG_DIR}" mb --region="${AWS_DEFAULT_REGION}" local/satysfi-playground
mc -C "${CONFIG_DIR}" policy set download local/satysfi-playground

echo 'Setup DONE'

wait
