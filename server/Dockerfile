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
RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo chef cook --release --recipe-path recipe.json

# Copy project files
COPY server/Cargo.toml ./Cargo.toml
COPY server/Cargo.lock ./Cargo.lock
COPY server/build.rs ./
COPY server/config.toml ./

# Remove workspace configuration from Cargo.toml if present
RUN sed -i '/\[workspace\]/d' ./Cargo.toml && \
    sed -i '/workspace\./d' ./Cargo.toml

# Copy actual source code
COPY server/src ./src

# Build the application with parallel compilation
RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo build --release --jobs $(nproc)

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
COPY --from=builder /usr/src/app/target/release/ultra_low_latency_server_chunked_parallel /app/server

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

# Environment variables for production (file paths only)
ENV SSL_CERT_PATH="/app/certs/localhost.crt" \
    SSL_PRIVATE_FILE="/app/certs/localhost.key" \
    DATA_PATH="/mnt/md/data"

# Run the server
CMD ["./server"]