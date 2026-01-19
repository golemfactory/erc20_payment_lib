ARG ERC20_PAYMENT_TAG=latest
FROM ghcr.io/golemfactory/erc20-processor:${ERC20_PAYMENT_TAG} as erc20-processor

FROM python:alpine
COPY --from=erc20-processor /usr/bin/erc20-processor /usr/bin/erc20-processor
COPY *.py .
COPY *.toml .

