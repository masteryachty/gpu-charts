# Build stage
FROM rust:1.82-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy only server-specific files
COPY server/Cargo.toml ./Cargo.toml
COPY server/Cargo.lock ./Cargo.lock
COPY server/build.rs ./
COPY server/config.toml ./

# Create dummy source files for dependency caching
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib() {}" > src/lib.rs

# Build dependencies
RUN cargo build --release --target x86_64-unknown-linux-gnu && \
    rm -rf src

# Copy actual source code
COPY server/src ./src

# Build the application
RUN touch src/main.rs src/lib.rs && \
    cargo build --release --target x86_64-unknown-linux-gnu

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash server

# Create necessary directories
RUN mkdir -p /app/certs /data && \
    chown -R server:server /app /data

WORKDIR /app

# Copy the built binary
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-gnu/release/ultra_low_latency_server_chunked_parallel /app/server

# Copy certificate generation script
COPY server/generate-certs.sh /tmp/

# Generate SSL certificates during build
RUN chmod +x /tmp/generate-certs.sh && \
    bash /tmp/generate-certs.sh && \
    rm /tmp/generate-certs.sh && \
    chown -R server:server /app/certs

# Switch to non-root user
USER server

# Expose HTTPS port
EXPOSE 8443

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD timeout 5s bash -c ':> /dev/tcp/127.0.0.1/8443' || exit 1

# Environment variables for production
# Note: These are just paths, not actual secrets
ENV SSL_CERT_PATH=/app/certs/localhost.crt
ENV SSL_KEY_PATH=/app/certs/localhost.key
ENV DATA_PATH=/data

# Run the server
CMD ["./server"]