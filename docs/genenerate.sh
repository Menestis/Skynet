#!/bin/bash

rm -r gen/src
java -jar ~/Softwares/openapi-generator-cli-6.0.0-20211025.061654-22.jar generate -i docs/openapi.yml -c docs/generator-config.json -g java -o gen
