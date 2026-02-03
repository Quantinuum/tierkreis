#!/usr/bin/env bash

numbits=$(cat $input_numbits_file)
openssl genrsa -out $output_private_key_file -aes128 -passout "file:$input_passphrase_file" $numbits
openssl rsa -in $output_private_key_file -passin "file:$input_passphrase_file" -pubout -out $output_public_key_file
