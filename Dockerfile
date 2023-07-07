ARG BUILD_PROFILE=release
ARG BUILD_TARGET=x86_64-unknown-linux-musl

FROM lukemathwalker/cargo-chef:latest AS chef
WORKDIR app
RUN apt-get update
RUN apt-get install musl-tools -y
ARG BUILD_TARGET
RUN rustup target add ${BUILD_TARGET}

FROM chef AS planner
COPY ./src ./src
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./examples ./examples
COPY ./crates ./crates
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
ARG BUILD_PROFILE
ARG BUILD_TARGET
RUN cargo chef cook --profile ${BUILD_PROFILE} --target ${BUILD_TARGET} --recipe-path recipe.json
# Build application
COPY ./src ./src
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./examples ./examples
COPY ./crates ./crates
RUN cargo build --profile ${BUILD_PROFILE} --target ${BUILD_TARGET}
RUN cargo build --profile ${BUILD_PROFILE} --target ${BUILD_TARGET} --examples


FROM nikolaik/python-nodejs:python3.10-nodejs18
RUN apt-get update
RUN apt-get install -y vim sqlite3
RUN pip install web3 python-dotenv

WORKDIR /app
COPY --from=ghcr.io/ufoscout/docker-compose-wait:latest /wait /wait
ARG BUILD_PROFILE
ARG BUILD_TARGET
COPY --from=builder /app/target/${BUILD_TARGET}/${BUILD_PROFILE}/erc20_processor /bin/erc20_processor
COPY --from=builder /app/target/${BUILD_TARGET}/${BUILD_PROFILE}/examples/generate_transfers /bin/generate_transfers
COPY ./config-payments-test.toml ./config-payments.toml
COPY ./scenarios/ .
CMD ["sh", "-c",  "/wait && /bin/erc20_processor"]








