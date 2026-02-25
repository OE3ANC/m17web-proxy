FROM ubuntu:24.04 AS build
LABEL authors="oe3anc"

RUN mkdir "app"
WORKDIR /app

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

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN mkdir ./tmp/
COPY . ./tmp/

WORKDIR /app/tmp

RUN cargo build --release

RUN cp /app/tmp/target/release/m17web-proxy /app/m17web-proxy

WORKDIR /app

RUN rm -rf ./tmp/

RUN ls -hal

FROM ubuntu:24.04

RUN apt update && apt install -y \
    libssl3t64 \
    libopendht3t64 \
    libopendht-c3t64 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/* /app/

ENTRYPOINT ["/app/m17web-proxy"]
