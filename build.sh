#!/bin/bash
./docs/genenerate.sh

docker run -v cargo-cache:/root/.cargo/registry -v "$PWD":/volume --rm -it clux/muslrust cargo build --release
cp target/x86_64-unknown-linux-musl/release/skynet out/skynet
docker build -t registry.aspaku.com/skynet/skynet -f Dockerfile out/