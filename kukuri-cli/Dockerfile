# Build stage
FROM rust:1.83-slim AS builder

# Install dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/kukuri-cli

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Build dependencies (cache layer)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source code
COPY src ./src

# Build the application
RUN touch src/main.rs
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash kukuri

# Copy binary from builder
COPY --from=builder /usr/src/kukuri-cli/target/release/kukuri-cli /usr/local/bin/kukuri-cli

# Set ownership
RUN chown kukuri:kukuri /usr/local/bin/kukuri-cli

# Switch to non-root user
USER kukuri

# Set working directory
WORKDIR /home/kukuri

# Expose default port
EXPOSE 11223

# Default command
ENTRYPOINT ["kukuri-cli"]
CMD ["bootstrap"]