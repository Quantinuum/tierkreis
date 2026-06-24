#! /usr/bin/env bash
in=$(cat | tr -d '"')
echo "\"Hello $in\""
