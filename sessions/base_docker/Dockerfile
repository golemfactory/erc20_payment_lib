FROM ethereum/client-go:v1.12.0 as go_client
FROM scx1332/web3_test_proxy:v0.2.0 as proxy_builder
FROM alpine:3.18.2

WORKDIR /runtime

COPY --from=go_client /usr/local/bin/geth /usr/local/bin/geth
COPY --from=proxy_builder /usr/local/bin/web3_test_proxy /usr/local/bin/web3_test_proxy
COPY genesis ./genesis
COPY start.sh ./

CMD ["sh", "start.sh"]
