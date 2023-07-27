FROM ethereum/client-go:v1.12.0 as go_client

FROM rust:1.71-alpine3.18 as proxy_builder
RUN apk add --no-cache musl-dev
RUN cargo install --target=x86_64-unknown-linux-musl ya_web3_proxy


FROM alpine:3.18.2

WORKDIR /runtime

COPY --from=go_client /usr/local/bin/geth /usr/local/bin/geth
COPY --from=proxy_builder /usr/local/cargo/bin/ya_web3_proxy /usr/local/bin/ya_web3_proxy
COPY genesis ./genesis
COPY start.sh ./

CMD ["sh", "start.sh"]
