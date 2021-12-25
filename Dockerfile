FROM debian

RUN groupadd -g 10001 -r dockergrp && useradd -r -g dockergrp -u 10001 dockeruser

RUN DEBIAN_FRONTEND=noninteractive apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

RUN update-ca-certificates

USER dockeruser
#COPY target/x86_64-unknown-linux-musl/release/skynet /skynet
COPY skynet /skynet

CMD ["/skynet"]