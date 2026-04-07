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
    libopendht-c-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN cargo build --release


FROM ubuntu:24.04

RUN apt update && apt install -y \
    libssl3t64 \
    libopendht3t64 \
    libopendht-c3t64 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/m17web-proxy /app/m17web-proxy

USER 1000:1000

ENTRYPOINT ["/app/m17web-proxy"]