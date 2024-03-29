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
export API_ENDPOINT="http://localhost:8080"
export S3_API_ENDPOINT='http://localhost:9000'
export S3_PUBLIC_ENDPOINT='http://localhost:9000/satysfi-playground'
export RUST_LOG='DEBUG'

# Object storage
podman run --rm -p 9000:9000 -p 9001:9001 \
    quay.io/minio/minio server /data --console-address ':9001' &

# API
nix run &

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

# Set up initial content
while ! ncat -v localhost 8080 < /dev/null
do
    echo 'Waiting for localhost:8080...'
    sleep 1
done
PERSIST_RESULT=$(./persist.sh "${API_ENDPOINT}" "$(dirname -- "${BASH_SOURCE[0]}")/examples/hello-playground/input.saty")
INDEX_PAGE_BUILD_ID=$(echo "${PERSIST_RESULT}" | grep -Eo '[a-z0-9]{64}' | tail -n 1)
export INDEX_PAGE_BUILD_ID
echo "up.sh: INDEX_PAGE_BUILD_ID=${INDEX_PAGE_BUILD_ID}"

# Frontend server. next dev does not work well with monaco, so we need to restart again and again...
(cd "$(dirname -- "${BASH_SOURCE[0]}")/frontend" || exit; npm run build && npm run start) &

# Load Docker image
export DOCKER_IMAGE_PATH
DOCKER_IMAGE_PATH=$(nix build '.#satysfi-docker' --no-link --json | jq --raw-output '.[0].outputs.out')
podman load < "${DOCKER_IMAGE_PATH}"

echo 'Setup DONE'

wait
