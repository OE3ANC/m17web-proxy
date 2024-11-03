FROM ubuntu:latest
LABEL authors="oe3anc"

RUN mkdir "app"
WORKDIR /app

RUN apt update && apt install curl build-essential -y

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"

COPY . .

RUN cargo build

RUN ls -hal

ENTRYPOINT ["/app/target/debug/m17web-proxy"]
