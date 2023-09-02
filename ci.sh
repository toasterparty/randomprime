#!/usr/bin/env bash

set -e -x
cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")"

echo "Installing Requirements"
python -m pip install json-schema-for-humans==0.45.1

echo "Creating Schema HTML"
rm -rf ./schema/build
generate-schema-doc --config-file ./schema/config.json ./schema/randomprime.schema.json ./schema/build/randomprime.html

cp ./schema/_config.yml ./schema/build
