#!/bin/bash

rm -r gen/src
rm -r docs/web
java -jar  ~/Softwares/openapi-generator/modules/openapi-generator-cli/target/openapi-generator-cli.jar generate -i docs/openapi.yml -c docs/generator-config.json -g java -o gen
java -jar ~/Softwares/openapi-generator/modules/openapi-generator-cli/target/openapi-generator-cli.jar generate -i docs/openapi.yml -o docs/web -g html2