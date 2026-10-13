# Multi-stage build: Rust builder, Debian slim runtime.
#
# Built on eopsin straight from GitHub by stacks/crater in unraid-config
# (`docker compose up -d --build` pulls the branch named there). The cache
# mounts keep the dependency build between deploys, so a rebuild after a
# small change takes about a minute rather than several.

FROM rust:1.98-slim-bookworm AS build
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p crater \
 && cp target/release/crater /usr/local/bin/crater

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
 && rm -rf /var/lib/apt/lists/*
COPY --from=build /usr/local/bin/crater /usr/local/bin/crater
ENV CRATER_BIND=0.0.0.0:8080 \
    CRATER_DATA_DIR=/data
VOLUME /data
EXPOSE 8080
HEALTHCHECK --interval=60s --timeout=5s CMD curl -fsS http://127.0.0.1:8080/api/health || exit 1
ENTRYPOINT ["/usr/local/bin/crater"]
