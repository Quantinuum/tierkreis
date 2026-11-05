#!/usr/bin/env bash
set -euo pipefail

worker_call_args=${1:?"usage: $0 WORKER_CALL_ARGS"}
error_path=$(jq -er '.error_path' "$worker_call_args")
trap 'status=$?; if (( status != 0 )); then printf "text_worker failed with exit code %d\n" "$status" > "$error_path"; fi' EXIT

function_name=$(jq -er '.function_name' "$worker_call_args")
case "$function_name" in
    sort_lines)
        input_source=$(jq -er '.inputs.source' "$worker_call_args")
        output_result=$(jq -er '.outputs.result' "$worker_call_args")
        LC_ALL=C sort <"$input_source" >"$output_result"
        ;;
    count_lines)
        input_source=$(jq -er '.inputs.source' "$worker_call_args")
        output_count=$(jq -er '.outputs.count' "$worker_call_args")
        count=$(wc -l <"$input_source")
        jq -n --argjson count "$count" '$count' >"$output_count"
        ;;
    *)
        printf 'unknown function: %s\n' "$function_name" >&2
        exit 1
        ;;
esac

done_path=$(jq -er '.done_path' "$worker_call_args")
touch "$done_path"
