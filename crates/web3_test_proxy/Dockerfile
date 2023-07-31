FROM rust:1.71-alpine3.18 as proxy_builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY src ./src
COPY Cargo.toml ./Cargo.toml
COPY frontend ./frontend
RUN cargo fetch --target=x86_64-unknown-linux-musl
RUN cargo build --profile=release-lto --target=x86_64-unknown-linux-musl

FROM alpine:3.18

COPY --from=proxy_builder /app/target/x86_64-unknown-linux-musl/release-lto/web3_test_proxy /usr/local/bin/web3_test_proxy
