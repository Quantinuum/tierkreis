#! /usr/bin/env bash

greeting=$(cat $input_greeting_file)
echo $TEST_FLAG $greeting >> $output_value_file
