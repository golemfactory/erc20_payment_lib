ARG BUILD_PROFILE=release
ARG BUILD_TARGET=x86_64-unknown-linux-musl

FROM lukemathwalker/cargo-chef:latest AS chef
WORKDIR app
RUN apt-get update
RUN apt-get install musl-tools -y
ARG BUILD_TARGET
RUN rustup target add ${BUILD_TARGET}

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
ARG BUILD_PROFILE
ARG BUILD_TARGET
RUN cargo chef cook --profile ${BUILD_PROFILE} --target ${BUILD_TARGET} --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --profile ${BUILD_PROFILE} --target ${BUILD_TARGET}


FROM alpine:latest
COPY --from=ghcr.io/ufoscout/docker-compose-wait:latest /wait /wait
ARG BUILD_PROFILE
ARG BUILD_TARGET
COPY --from=builder /app/target/${BUILD_TARGET}/${BUILD_PROFILE}/erc20_processor /bin/erc20_processor

CMD ["sh", "-c",  "/wait && /bin/erc20_processor"]








