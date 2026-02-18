FROM rust:1.89-bookworm AS builder
WORKDIR /app

COPY Cargo.toml ./
COPY crates ./crates
COPY migrations ./migrations

RUN cargo build --release -p cn-cli

FROM debian:bookworm-slim
RUN apt-get update     && apt-get install -y --no-install-recommends ca-certificates     && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cn /usr/local/bin/cn

ENTRYPOINT ["cn"]
