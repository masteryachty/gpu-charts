# Multi-stage build for GPU Charts Server
# Works both locally and from api.rednax.io

# Chef stage for better dependency caching
FROM rust:1.82-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /usr/src/app

# Planner stage
FROM chef AS planner
COPY server/Cargo.toml ./Cargo.toml
COPY server/Cargo.lock ./Cargo.lock
COPY server/build.rs ./
COPY server/config.toml ./

# Remove workspace configuration from Cargo.toml if present
RUN sed -i '/\[workspace\]/d' ./Cargo.toml && \
    sed -i '/workspace\./d' ./Cargo.toml

RUN cargo chef prepare --recipe-path recipe.json

# Build stage
FROM chef AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /usr/src/app/recipe.json recipe.json

# Build dependencies with cargo-chef (much better caching)
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
RUN cargo chef cook --release --recipe-path recipe.json

# Copy all project files
COPY server/Cargo.toml ./Cargo.toml
COPY server/Cargo.lock ./Cargo.lock
COPY server/build.rs ./
COPY server/config.toml ./

# Remove workspace configuration
RUN sed -i '/\[workspace\]/d' ./Cargo.toml && \
    sed -i '/workspace\./d' ./Cargo.toml

# Copy source code
COPY server/src ./src

# Build the application
RUN cargo build --release --jobs $(nproc)

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash server

# Create necessary directories
RUN mkdir -p /app/certs /data && \
    chown -R server:server /app /data

WORKDIR /app

# Copy the built binary
COPY --from=builder /usr/src/app/target/release/ultra_low_latency_server_chunked_parallel /app/server

# Conditionally copy certificates if they exist (for TLS mode)
# Using a workaround since COPY doesn't support conditional operations
RUN mkdir -p /app/certs && chown -R server:server /app/certs

# Copy certificates directory - will be empty if no certs
COPY server/certs /tmp/certs

# Move certificates if they exist
RUN if [ -f /tmp/certs/api.rednax.io.crt ] && [ -f /tmp/certs/api.rednax.io.key ]; then \
        echo "Certificates found - TLS mode available"; \
        cp /tmp/certs/api.rednax.io.crt /app/certs/localhost.crt; \
        cp /tmp/certs/api.rednax.io.key /app/certs/localhost.key; \
        chmod 644 /app/certs/localhost.crt; \
        chmod 600 /app/certs/localhost.key; \
    else \
        echo "No certificates found - HTTP mode only"; \
    fi && \
    rm -rf /tmp/certs

# Set permissions
RUN chmod +x /app/server && \
    chown -R server:server /app

# Expose both HTTP and HTTPS ports
EXPOSE 8080 8443

# Environment variables
# Optimized for Cloudflare Tunnel by default
ENV SSL_CERT_PATH="/app/certs/localhost.crt" \
    SSL_PRIVATE_FILE="/app/certs/localhost.key" \
    DATA_PATH="/mnt/md/data" \
    USE_TLS="false" \
    PORT="8443"

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8443/api/symbols || exit 1

# Switch to non-root user
USER server

# Run the server directly
CMD ["/app/server"]