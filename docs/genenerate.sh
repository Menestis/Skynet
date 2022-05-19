#!/bin/bash

GENERATOR="java -jar $HOME/Softwares/openapi-generator/modules/openapi-generator-cli/target/openapi-generator-cli.jar"

echo "Generating java clients"
rm -r gen/java/
$GENERATOR generate -i docs/openapi.yml -c docs/java-generator-config.json -g java -o gen/java

echo "Generating web documentation"
rm -r gen/rust/
$GENERATOR  generate -i docs/openapi.yml -c docs/rust-generator-config.json -o gen/rust -g rust

echo "Generating web documentation"
rm -r docs/web
$GENERATOR generate -i docs/openapi.yml -o docs/web -g html2