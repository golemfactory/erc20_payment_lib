FROM rust:1.71-alpine3.18 as proxy_builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . ./
RUN cargo fetch --target=x86_64-unknown-linux-musl
RUN cargo build --profile=release-lto --target=x86_64-unknown-linux-musl

