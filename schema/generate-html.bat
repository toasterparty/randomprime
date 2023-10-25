@echo off
cd /d "%~dp0"

if not exist build (
    mkdir build
)

py -m pip install --upgrade pip
py -m pip install json-schema-for-humans
generate-schema-doc --config-file .\config.json .\randomprime.schema.json .\build\randomprime.html
