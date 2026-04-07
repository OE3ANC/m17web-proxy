FROM ubuntu:24.04
LABEL authors="oe3anc"

#### TODO -> Sync changes with build Dockerfile!

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
