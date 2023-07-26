FROM ethereum/client-go:v1.12.0 as go_client
FROM scx1332/geth as base

FROM rust as proxy_builder
RUN apt-get update
RUN apt-get install -y musl-tools
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install --target=x86_64-unknown-linux-musl ya_web3_proxy


FROM alpine as runner

WORKDIR /runtime

COPY --from=go_client /usr/local/bin/geth /usr/local/bin/geth
COPY --from=base /runtime/tmp /runtime/tmp
COPY --from=proxy_builder /usr/local/cargo/bin/ya_web3_proxy /usr/local/bin/ya_web3_proxy
COPY start.sh ./

