FROM ubuntu:24.04 as build
LABEL authors="oe3anc"

RUN mkdir "app"
WORKDIR /app

RUN apt update && apt install curl build-essential pkg-config libssl-dev -y

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN mkdir ./tmp/
COPY . ./tmp/

WORKDIR /app/tmp

RUN cargo build

RUN cp /app/tmp/target/debug/m17web-proxy /app/m17web-proxy

RUN curl -o /app/reflector.json https://dvref.com/mrefd/json/?format=json

WORKDIR /app

RUN rm -rf ./tmp/

RUN ls -hal

FROM ubuntu:24.04

RUN apt update && apt install curl build-essential pkg-config libssl-dev -y

COPY --from=build /app/* /app/

ENTRYPOINT ["/app/m17web-proxy"]
