#!/usr/bin/env bash
# Run this script from the development shell.
# This script performs an API call against /persist using the given file and returns URLs for the generated files

set -euo pipefail

S3_URL=$(
    base64 -w 0 "$2" | \
    jq --raw-input '{"source":.}' | \
    curl \
        -d @- \
        -H 'Content-Type: application/json' \
        "$1/persist" | \
    jq --raw-output '.s3_url'
)
echo "document: ${S3_URL}/document.pdf"
echo "  stdout: ${S3_URL}/stdout.txt"
echo "  stderr: ${S3_URL}/stderr.txt"