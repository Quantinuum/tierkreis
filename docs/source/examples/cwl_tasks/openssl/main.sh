#!/usr/bin/env bash
set -euo pipefail

worker_call_args=${1:?"usage: $0 WORKER_CALL_ARGS"}
error_path=$(jq -er '.error_path' "$worker_call_args")
trap 'status=$?; if (( status != 0 )); then printf "openssl_worker failed with exit code %d\n" "$status" > "$error_path"; fi' EXIT

function_name=$(jq -er '.function_name' "$worker_call_args")
if [[ $function_name != "genrsa" ]]; then
    printf 'unknown function: %s\n' "$function_name" >&2
    exit 1
fi

input_numbits=$(jq -er '.inputs.numbits' "$worker_call_args")
input_passphrase=$(jq -er '.inputs.passphrase' "$worker_call_args")
output_private_key=$(jq -er '.outputs.private_key' "$worker_call_args")
output_public_key=$(jq -er '.outputs.public_key' "$worker_call_args")
done_path=$(jq -er '.done_path' "$worker_call_args")

numbits=$(<"$input_numbits")
openssl genrsa -out "$output_private_key" -aes128 -passout "file:$input_passphrase" "$numbits"
openssl rsa -in "$output_private_key" -passin "file:$input_passphrase" -pubout -out "$output_public_key"
touch "$done_path"
