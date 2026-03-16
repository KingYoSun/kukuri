FROM rust:1.89-bookworm AS builder

WORKDIR /workspace

ARG TARGET_PACKAGE

COPY Cargo.toml Cargo.lock ./
COPY xtask ./xtask
COPY crates ./crates

RUN test -n "$TARGET_PACKAGE"
RUN cargo build --release -p "$TARGET_PACKAGE"

FROM debian:bookworm-slim AS runtime

ARG TARGET_BIN

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN test -n "$TARGET_BIN"
COPY --from=builder /workspace/target/release/ /tmp/release/
RUN cp "/tmp/release/${TARGET_BIN}" /usr/local/bin/app

ENTRYPOINT ["/usr/local/bin/app"]
