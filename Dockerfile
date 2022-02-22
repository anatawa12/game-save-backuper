# syntax=docker.io/docker/dockerfile:1.3.0
FROM rust:1 as builder

WORKDIR /project/

COPY Cargo* ./
COPY src src

RUN cargo build --release

FROM gcr.io/distroless/cc-debian11

COPY --from=builder /project/target/release/game-save-backuper /game-save-backuper
ARG TARGETARCH
ARG TARGETVARIANT
ADD --chmod=555 "https://api.anatawa12.com/short/tini-download?arch=${TARGETARCH}&variant=${TARGETVARIANT}" /tini

ENTRYPOINT ["/tini", "--", "/game-save-backuper"]
