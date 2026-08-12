# syntax=docker/dockerfile:1

FROM node:22-bookworm AS web
WORKDIR /web
COPY web/package.json web/package-lock.json* ./
RUN npm ci || npm install
COPY web/ ./
RUN npm run build

FROM rust:1.85-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && useradd -r -u 10001 -m logdb
WORKDIR /app
COPY --from=builder /app/target/release/logdb /usr/local/bin/logdb
COPY --from=web /web/dist /app/web/dist
ENV WEB_DIR=/app/web/dist \
    BIND=0.0.0.0:8080 \
    PUBLIC_BASE_URL=http://localhost:8080
VOLUME ["/data"]
EXPOSE 8080
USER logdb
ENTRYPOINT ["/usr/local/bin/logdb"]
