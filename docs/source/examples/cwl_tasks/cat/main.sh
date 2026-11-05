#!/usr/bin/env bash
set -euo pipefail

worker_call_args=${1:?"usage: $0 WORKER_CALL_ARGS"}
error_path=$(jq -er '.error_path' "$worker_call_args")
workdir=
trap 'status=$?; if [[ -n $workdir ]]; then rm -rf -- "$workdir"; fi; if (( status != 0 )); then printf "cat_worker failed with exit code %d\n" "$status" > "$error_path"; fi' EXIT

function_name=$(jq -er '.function_name' "$worker_call_args")
if [[ $function_name != "concatenate" ]]; then
    printf 'unknown function: %s\n' "$function_name" >&2
    exit 1
fi

input_source=$(jq -er '.inputs.source' "$worker_call_args")
input_options=$(jq -er '.inputs.options' "$worker_call_args")
output_result=$(jq -er '.outputs.result' "$worker_call_args")
done_path=$(jq -er '.done_path' "$worker_call_args")

jq -e 'type == "array" and all(.[]; type == "string" and (contains("\u0000") | not))' \
    "$input_options" >/dev/null
options=()
while IFS= read -r -d '' option; do
    options+=("$option")
done < <(jq -j '.[] | ., "\u0000"' "$input_options")

workdir=$(mktemp -d)
cp -- "$input_source" "$workdir/input.txt"
(
    cd -- "$workdir"
    cat "${options[@]+"${options[@]}"}" input.txt >output.txt
)
cp -- "$workdir/output.txt" "$output_result"
touch "$done_path"
