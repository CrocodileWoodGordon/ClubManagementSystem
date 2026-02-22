# syntax=docker/dockerfile:1.6

FROM rust:1.75-slim AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/migrations ./migrations
COPY backend/src ./src

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/backend /usr/local/bin/club-backend

ENV RUST_BACKTRACE=1

EXPOSE 8080

CMD ["club-backend"]
