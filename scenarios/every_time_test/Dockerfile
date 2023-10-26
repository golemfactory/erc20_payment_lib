ARG ERC20_PAYMENT_TAG=latest
FROM ghcr.io/golemfactory/erc20_processor:${ERC20_PAYMENT_TAG} as erc20_processor

FROM python:alpine
COPY --from=erc20_processor /usr/bin/erc20_processor /usr/bin/erc20_processor
COPY *.py .
COPY *.toml .

