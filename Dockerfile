FROM ubuntu:latest
LABEL authors="oe3anc"

RUN mkdir "app"
WORKDIR /app
COPY ./target/debug/m17rx /app/m17rx

ENTRYPOINT ["/app/m17rx"]

