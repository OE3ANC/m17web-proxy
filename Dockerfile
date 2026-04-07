FROM ubuntu:24.04 AS build
LABEL authors="oe3anc"

#### TODO -> Sync changes with devcontainer Dockerfile!

RUN apt update && apt install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    libclang-dev \
    clang \
    libopendht-dev \
    libopendht-c-dev

RUN mkdir -p /app/tmp/
COPY --chown=1000:1000 . /app/tmp/

WORKDIR /app/tmp/

USER 1000:1000

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
RUN cargo build --release

RUN cp /app/tmp/target/release/m17web-proxy /app/m17web-proxy

WORKDIR /app

RUN rm -rf ./tmp/

FROM ubuntu:24.04

RUN apt update && apt install -y \
    libssl3t64 \
    libopendht3t64 \
    libopendht-c3t64 \
    && rm -rf /var/lib/apt/lists/*

USER 1000:1000

COPY --chown=1000:1000 --from=build /app/* /app/

ENTRYPOINT ["/app/m17web-proxy"]
