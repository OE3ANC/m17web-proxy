FROM ubuntu:latest
LABEL authors="oe3anc"

RUN mkdir "app"
WORKDIR /app
COPY ./target/debug/m17web-proxy /app/m17web-proxy

ENTRYPOINT ["/app/m17web-proxy"]

