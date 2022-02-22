# syntax=docker.io/docker/dockerfile:1.3.0
FROM gcr.io/distroless/cc-debian11

ARG TARGETARCH
ARG TARGETVARIANT
COPY --chmod=555 "./outputs/built-${TARGETARCH}/game-save-backuper" /game-save-backuper
ADD --chmod=555 "https://api.anatawa12.com/short/tini-download?arch=${TARGETARCH}&variant=${TARGETVARIANT}" /tini

ENTRYPOINT ["/tini", "--", "/game-save-backuper"]
