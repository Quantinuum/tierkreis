#! /usr/bin/env bash

greeting=$(cat $input_greeting_file | tr -d '"')
echo "\"$TEST_FLAG $greeting\"" >> $output_value_file
